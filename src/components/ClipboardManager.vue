<script setup>
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { register } from "@tauri-apps/plugin-global-shortcut";
import ClipboardItem from "./ClipboardItem.vue";

const clipboardItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);
const showLoadingStatus = ref(false);
let loadingTimer = null;
const selectedIndex = ref(-1); // -1 means no item selected
const currentPageOffset = ref(0);
const itemsPerPage = 10;
const clipboardManager = ref(null);
const totalItems = ref(0);
let resizeObserver = null;

// Modal state
const showDirModal = ref(false);
const mismatchDirs = ref({ current: "", expected: "" });

// Cycling mode state
const isCycling = ref(false);
const isCtrlPressed = ref(true);

// Get the currently selected item
const selectedItem = computed(() => {
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        return clipboardItems.value[selectedIndex.value];
    }
    return null;
});

// Computed placeholder for search input
const searchPlaceholder = computed(() => {
    if (currentPageOffset.value > 0) {
        const startOffset = currentPageOffset.value + 1;
        const endOffset = currentPageOffset.value + clipboardItems.value.length;
        return `Search ${totalItems.value} items (showing ${startOffset}-${endOffset})`;
    } else {
        return `Search ${totalItems.value} items`;
    }
});

// Check directory mismatch
async function checkDataDirectory() {
    try {
        // Wait a bit for the service to be ready
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        const currentDir = await invoke('get_sidecar_dir');
        const expectedDir = await invoke('get_app_data_dir');
        
        console.log('Directory check:', { currentDir, expectedDir });
        
        if (currentDir && expectedDir && currentDir !== expectedDir) {
            mismatchDirs.value = { current: currentDir, expected: expectedDir };
            showDirModal.value = true;
            // Adjust window size to show modal
            const window = getCurrentWindow();
            await window.setSize(new LogicalSize(400, 400));
        }
    } catch (e) {
        console.error("Failed to check directory:", e);
    }
}

async function handleDirChoice(choice) {
    try {
        const path = mismatchDirs.value.expected;
        if (choice === 'create') {
            // Option A: Create new directory (Update path)
            await invoke('set_sidecar_dir', { mode: 'update', path });
        } else if (choice === 'import') {
            // Option B: Import old directory (Move data)
            await invoke('set_sidecar_dir', { mode: 'move', path });
        }
        // Option C: Continue (do nothing)
        
        showDirModal.value = false;
        // Refresh list as directory might have changed
        loadRecentItems();
        resizeWindowToFitContent();
    } catch (e) {
        console.error("Failed to set directory:", e);
        alert("Failed to update directory: " + e);
    }
}

// Start loading with delay for status bar
function startLoading() {
    isLoading.value = true;
    showLoadingStatus.value = false;

    // Clear any existing timer
    if (loadingTimer) {
        clearTimeout(loadingTimer);
    }

    // Show loading status after 300ms
    loadingTimer = setTimeout(() => {
        showLoadingStatus.value = true;
    }, 300);
}

// Stop loading and clear timer
function stopLoading() {
    isLoading.value = false;
    showLoadingStatus.value = false;

    if (loadingTimer) {
        clearTimeout(loadingTimer);
        loadingTimer = null;
    }
}

// Search for clipboard items
async function searchItems(query) {
    try {
        startLoading();
        if (!query.trim()) {
            await loadRecentItems();

            // Unregister global shortcut when component mounts (window opens)
            await unregisterGlobalShortcut();
            return;
        }
        const jsonStr = await invoke("get_history", { query, limit: itemsPerPage, offset: 0 });
        const rawItems = JSON.parse(jsonStr);
        
        const items = rawItems.map(item => ({
            id: item.index,
            text: item.summary,
            timestamp: new Date(item.date).getTime(),
            byteSize: item.size,
            copies: item.copyCount || 0,
            firstCopied: item.firstDate ? new Date(item.firstDate).getTime() / 1000 : new Date(item.date).getTime() / 1000,
            data: item.data,
            formats: {
                imageData: item.type === "image",
                files: item.type === "file" ? [item.summary] : [],
            }
        }));

        clipboardItems.value = items;
        currentPageOffset.value = 0;
        selectedIndex.value = -1;

        // Resize window after search results load
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to search items:", error);
    } finally {
        stopLoading();
    }
}

// Load total item count
async function loadTotalItems() {
    try {
        const count = await invoke("db_get_count");
        totalItems.value = count;
    } catch (e) {
        console.error("Failed to load total items:", e);
        totalItems.value = 0;
    }
}

// Load recent items
async function loadRecentItems(offset = 0) {
    try {
        startLoading();
        // Use offset and limit for efficient pagination
        const jsonStr = await invoke("get_history", {
            limit: itemsPerPage,
            offset: offset,
        });
        const rawItems = JSON.parse(jsonStr);
        
        const items = rawItems.map(item => ({
            id: item.index,
            text: item.summary,
            timestamp: new Date(item.date).getTime(),
            byteSize: item.size,
            copies: item.copyCount || 0,
            firstCopied: item.firstDate ? new Date(item.firstDate).getTime() / 1000 : new Date(item.date).getTime() / 1000,
            data: item.data,
            formats: {
                imageData: item.type === "image",
                files: item.type === "file" ? [item.summary] : [],
            }
        }));

        console.log(`=== LOADED ITEMS (Offset: ${offset}) ===`);
        clipboardItems.value = items;
        currentPageOffset.value = offset;
        
        // If we just loaded a new page, select the first item if moving down, or last if moving up?
        // Actually, handleArrowDown/Up manages selection index.
        // If we are just refreshing (offset 0), deselect.
        if (offset === 0 && selectedIndex.value === -1) {
             // Keep it -1
        }

        // Resize window after content loads
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to load recent items:", error);
    } finally {
        stopLoading();
    }
}

// Watch search query
watch(
    searchQuery,
    (newQuery) => {
        selectedIndex.value = -1;
        currentPageOffset.value = 0;
        searchItems(newQuery);
    },
    { debounce: 300 },
);

// Delete clipboard item
async function deleteItem(id) {
    try {
        await invoke("delete_item", { selector: id.toString() });
        clipboardItems.value = clipboardItems.value.filter(
            (item) => item.id !== id,
        );
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

// Event listeners
document.addEventListener("keydown", (e) => {
    handleKeyDown(e);

    if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        const key = e.key;
        let itemIndex = null;
        if (key >= "1" && key <= "9") itemIndex = parseInt(key) - 1;
        else if (key === "0") itemIndex = 9;

        if (itemIndex !== null && clipboardItems.value[itemIndex]) {
            e.preventDefault();
            pasteItemToSystem(clipboardItems.value[itemIndex]);
        }
    }

    if (!isCycling.value) {
        if (e.key === "ArrowDown") {
            e.preventDefault();
            handleArrowDown();
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            handleArrowUp();
        } else if (e.key === "Enter") {
            e.preventDefault();
            handleEnter();
        }
    }
});

document.addEventListener("keyup", (e) => {
    handleKeyUp(e);
    if (e.key === "Escape") {
        if (showDirModal.value) {
            // Treat escape as "Continue"
            showDirModal.value = false;
            loadRecentItems();
            resizeWindowToFitContent();
            return;
        }
        let win = getCurrentWindow();
        win.hide();
        registerGlobalShortcut();
    }
});

async function handleArrowDown() {
    if (clipboardItems.value.length === 0) return;
    
    if (selectedIndex.value === clipboardItems.value.length - 1) {
        // We are at the bottom of the current list.
        // Load next page.
        const newOffset = currentPageOffset.value + 1;
        await loadRecentItems(newOffset);
        
        if (clipboardItems.value.length > 0) {
             // Select the last item of the new list (which is effectively the same visual position if we shifted)
             // Wait, if we shift by 1, the item at index 9 becomes index 8.
             // The user said "Only load the next item and discard the last item".
             // If we use offset += 1, we effectively scroll down by 1 item.
             // The list content shifts up. The item at index 9 is now the item that was at index 10 (globally).
             // So we should keep selectedIndex at the bottom (9).
             selectedIndex.value = clipboardItems.value.length - 1;
        } else {
             // End of list, revert
             await loadRecentItems(currentPageOffset.value - 1);
             selectedIndex.value = clipboardItems.value.length - 1;
        }
    } else {
        selectedIndex.value = selectedIndex.value + 1;
    }
}

async function handleArrowUp() {
    if (clipboardItems.value.length === 0) return;
    
    if (selectedIndex.value === 0) {
        if (currentPageOffset.value > 0) {
            // Load previous page (shift up by 1)
            const newOffset = Math.max(0, currentPageOffset.value - 1);
            await loadRecentItems(newOffset);
            // Keep selection at top
            selectedIndex.value = 0;
        }
    } else {
        selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    }
}

function handleEnter() {
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        pasteItemToSystem(clipboardItems.value[selectedIndex.value]);
    }
}

async function pasteItemToSystem(item) {
    try {
        await invoke("paste_item", { selector: item.id.toString() });
        await loadRecentItems(currentPageOffset.value);
    } catch (error) {
        console.error("Failed to inject item:", error);
    }
}

function resetSelection() {
    selectedIndex.value = -1;
    searchQuery.value = "";
    currentPageOffset.value = 0;
}

function formatFirstCopied(firstCopied) {
    const date = new Date(firstCopied * 1000);
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const hours = date.getHours();
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const ampm = hours >= 12 ? "PM" : "AM";
    const displayHours = hours % 12 || 12;
    return `${month}/${day} @ ${displayHours}:${minutes}${ampm}`;
}

function formatByteSize(bytes) {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}K`;
    return `${Math.round(bytes / (1024 * 1024))}M`;
}

function countWords(text) {
    if (!text) return 0;
    return text.trim().split(/\s+/).filter((word) => word.length > 0).length;
}

function getItemInfo(item) {
    if (!item) return null;
    if (item.formats?.imageData) return { type: "image", size: formatByteSize(item.byteSize), label: "Image" };
    if (item.formats?.files && item.formats.files.length > 0) return { type: "files", size: `${item.formats.files.length} file${item.formats.files.length > 1 ? "s" : ""}`, label: "Files" };
    const wordCount = countWords(item.text || "");
    return { type: "text", size: `${wordCount} words`, label: "Text" };
}

async function resizeWindowToFitContent() {
    if (!clipboardManager.value || showDirModal.value) return;
    try {
        await nextTick();
        const rect = clipboardManager.value.getBoundingClientRect();
        const contentHeight = rect.height;
        const minHeight = 200;
        const maxHeight = 600;
        const finalHeight = Math.max(minHeight, Math.min(maxHeight, contentHeight));
        const window = getCurrentWindow();
        await window.setSize(new LogicalSize(400, finalHeight));
    } catch (error) {
        console.error("Failed to resize window:", error);
    }
}

async function unregisterGlobalShortcut() {
    try {
        await invoke("unregister_main_shortcut");
    } catch (error) {
        console.error("Failed to unregister global shortcut:", error);
    }
}

async function registerGlobalShortcut() {
    try {
        await invoke("register_main_shortcut");
    } catch (error) {
        console.error("Failed to register global shortcut:", error);
    }
}

function startCyclingMode() {
    isCycling.value = true;
    if (clipboardItems.value.length > 0) selectedIndex.value = 0;
}

function cycleToNext() {
    if (!isCycling.value || clipboardItems.value.length === 0) return;
    selectedIndex.value = (selectedIndex.value + 1) % clipboardItems.value.length;
}

async function endCycling() {
    if (!isCycling.value) return;
    isCycling.value = false;
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        await pasteItemToSystem(clipboardItems.value[selectedIndex.value]);
    }
    await registerGlobalShortcut();
    const window = getCurrentWindow();
    window.hide();
}

function handleKeyDown(e) {
    if (e.key === "Control" && !isCtrlPressed.value) isCtrlPressed.value = true;
    if (isCtrlPressed.value && e.key === "p") {
        e.preventDefault();
        if (!isCycling.value) startCyclingMode();
        else cycleToNext();
    }
}

function handleKeyUp(e) {
    if (e.key === "Control") {
        isCtrlPressed.value = false;
        if (isCycling.value) endCycling();
    }
}

onMounted(async () => {
    const unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
        if (!focused) {
            if (isCycling.value) {
                isCycling.value = false;
                isCtrlPressed.value = false;
            }
            registerGlobalShortcut();
            resetSelection();
            loadRecentItems();
        } else {
            unregisterGlobalShortcut();
            if (!isCycling.value) {
                resetSelection();
                document.querySelector(".search-input")?.focus();
            }
            isCtrlPressed.value = true;
            if (!showDirModal.value) resizeWindowToFitContent();
        }
    });

    await loadTotalItems();
    await loadRecentItems();
    await checkDataDirectory(); // Check dir on startup

    if (clipboardManager.value) {
        resizeObserver = new ResizeObserver(() => {
            resizeWindowToFitContent();
        });
        resizeObserver.observe(clipboardManager.value);
    }

    // Removed polling as per user request
    // startPolling();

    return () => {
        // stopPolling();
        unlistenFocus();
        if (resizeObserver && clipboardManager.value) resizeObserver.disconnect();
        registerGlobalShortcut();
    };
});
</script>

<template>
    <div class="clipboard-manager" ref="clipboardManager">
        <!-- Modal for directory mismatch -->
        <div v-if="showDirModal" class="modal-overlay">
            <div class="modal">
                <h3>Storage Location</h3>
                <p>The clipboard data directory differs from the recommended application support directory.</p>
                <div class="paths">
                    <div class="path-item">
                        <strong>Current:</strong> {{ mismatchDirs.current }}
                    </div>
                    <div class="path-item">
                        <strong>Recommended:</strong> {{ mismatchDirs.expected }}
                    </div>
                </div>
                <div class="modal-actions">
                    <button @click="handleDirChoice('create')">
                        Create new in Recommended
                    </button>
                    <button @click="handleDirChoice('import')">
                        Import to Recommended
                    </button>
                    <button @click="handleDirChoice('continue')" class="secondary">
                        Continue using Current
                    </button>
                </div>
            </div>
        </div>

        <!-- Search bar -->
        <div class="search-container">
            <input
                v-model="searchQuery"
                type="text"
                :placeholder="searchPlaceholder"
                class="search-input"
                autofocus
            />
        </div>

        <!-- Clipboard items list -->
        <div class="items-container">
            <div
                v-if="clipboardItems?.length === 0 && !isLoading"
                class="empty-state"
            >
                <div class="empty-icon">📋</div>
                <p>
                    {{
                        searchQuery
                            ? "No results"
                            : "Copy something to get started"
                    }}
                </p>
            </div>

            <div v-else class="clipboard-list">
                <ClipboardItem
                    v-for="(item, index) in clipboardItems"
                    :key="item.id"
                    :item="{ ...item, index }"
                    :selected="index === selectedIndex"
                    @mouseenter="selectedIndex = index"
                    @delete="deleteItem(item.id)"
                />
            </div>
        </div>

        <!-- Status bar -->
        <div v-if="showLoadingStatus || selectedItem" class="status-bar">
            <div v-if="showLoadingStatus" class="status-item loading-status">
                <div class="spinner"></div>
                <span class="status-value">Loading...</span>
            </div>

            <template v-else-if="selectedItem">
                <div class="status-item">
                    <span class="status-value">{{ formatFirstCopied(selectedItem.firstCopied) }}</span>
                </div>
                <div class="status-item">
                    <span class="status-value">{{ selectedItem.copies }}</span>
                </div>
                <div class="status-item">
                    <span class="status-value">{{ getItemInfo(selectedItem)?.size }}</span>
                </div>
            </template>
        </div>
    </div>
</template>

<style lang="less">
.clipboard-manager {
    display: flex;
    flex-direction: column;
    font-family: system-ui, sans-serif;
    font-weight: normal;
    gap: 10px;
    padding: 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    min-height: 200px; /* Ensure space for modal */

    .search-container,
    .items-container {
        display: flex;
        flex-direction: column;
    }

    .search-container {
        margin-top: 3px;
    }

    .clipboard-list {
        padding-top: 10px;
        display: flex;
        flex-direction: column;
        gap: 1px;

        .clipboard-item {
            height: 23px;
            overflow: hidden;
            cursor: default;
            font-size: 0.8em;
            display: flex;
            justify-content: space-between;
            gap: 10px;
            align-items: center;
            border-radius: 4px;
            padding: 1px 5px;
            color: var(--text-primary);

            .info {
                opacity: 0.6;
                color: var(--text-secondary);
            }

            &.is-selected {
                background: var(--accent);
                color: var(--accent-text);
                .info {
                    color: var(--accent-text);
                    opacity: 0.8;
                }
            }
        }
        
        .clipboard-item:has(img) {
            height: 80px;
            padding-top: 4px;
            padding-bottom: 4px;
            img {
                height: calc(100% - 8px);
            }
        }
    }
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
    padding: 20px;
    backdrop-filter: blur(5px);
}

.modal {
    background: white;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 4px 6px rgba(0,0,0,0.1);
    width: 100%;
    max-width: 400px;
    color: #333;

    h3 {
        margin-top: 0;
    }

    .paths {
        background: #f5f5f5;
        padding: 10px;
        border-radius: 4px;
        font-size: 0.8em;
        margin: 15px 0;
        word-break: break-all;
        
        .path-item {
            margin-bottom: 8px;
            &:last-child { margin-bottom: 0; }
        }
    }

    .modal-actions {
        display: flex;
        flex-direction: column;
        gap: 8px;

        button {
            padding: 8px;
            border-radius: 4px;
            border: none;
            background: var(--accent);
            color: white;
            cursor: pointer;
            font-weight: bold;
            
            &:hover {
                filter: brightness(1.1);
            }

            &.secondary {
                background: #e0e0e0;
                color: #333;
            }
        }
    }
}

.search-input {
    background: var(--bg-input);
    border: 0.5px solid var(--border-light);
    border-radius: 5px;
    padding: 5px 8px;
    font-family: system-ui;
    box-shadow: var(--shadow-light);
    color: var(--text-primary);
    width: 100%;

    &::placeholder {
        color: var(--text-secondary);
        opacity: 0.7;
    }

    &:focus {
        outline: none;
        border: none;
        box-shadow: 0 0 0 3px var(--accent-transparent);
    }
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

.empty-state {
    text-align: center;
    color: var(--text-secondary);
}

.empty-icon {
    font-size: 32px;
    filter: grayscale(0.3);
}

.status-bar {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    background: var(--bg-status);
    color: var(--text-secondary);
    border-radius: 4px;
    font-size: 0.75em;
    margin-top: auto;
    margin-bottom: 4px;
    flex-shrink: 0;
    height: 20px;
    line-height: 20px;
}

.status-item {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    justify-content: center;
}

.status-value {
    font-weight: 300;
    color: var(--text-secondary);
}

.loading-status {
    justify-content: center;
    gap: 6px;
}

.loading-status .spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid var(--border-color);
    border-radius: 50%;
    border-top: none;
    animation: spin 1s linear infinite;
}
</style>
