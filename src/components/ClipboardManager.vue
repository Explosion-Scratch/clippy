<script setup>
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
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
let pollingInterval = null;
let lastKnownId = null;

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
        if (localStorage.getItem('ignoreDirMismatch') === 'true') return;
        await new Promise(resolve => setTimeout(resolve, 1000));
        const currentDir = await invoke('get_sidecar_dir');
        const expectedDir = await invoke('get_app_data_dir');
        
        if (currentDir && expectedDir && currentDir !== expectedDir) {
            mismatchDirs.value = { current: currentDir, expected: expectedDir };
            showDirModal.value = true;
            const window = getCurrentWindow();
            await window.setSize(new LogicalSize(400, 400));
        }
    } catch (e) {
        console.error("Failed to check directory:", e);
    }
}

async function handleDirChoice(choice) {
    try {
        if (choice === 'continue') {
            localStorage.setItem('ignoreDirMismatch', 'true');
            showDirModal.value = false;
            loadItems();
            resizeWindowToFitContent();
            return;
        }
        const path = mismatchDirs.value.expected;
        if (choice === 'create') {
            await invoke('set_sidecar_dir', { mode: 'update', path });
        } else if (choice === 'import') {
            await invoke('set_sidecar_dir', { mode: 'move', path });
        }
        showDirModal.value = false;
        loadItems();
        resizeWindowToFitContent();
    } catch (e) {
        console.error("Failed to set directory:", e);
        alert("Failed to update directory: " + e);
    }
}

function startLoading() {
    isLoading.value = true;
    showLoadingStatus.value = false;
    if (loadingTimer) clearTimeout(loadingTimer);
    loadingTimer = setTimeout(() => { showLoadingStatus.value = true; }, 300);
}

function stopLoading() {
    isLoading.value = false;
    showLoadingStatus.value = false;
    if (loadingTimer) { clearTimeout(loadingTimer); loadingTimer = null; }
}

async function loadItems(offset = 0) {
    try {
        startLoading();
        const query = searchQuery.value.trim();
        const jsonStr = await invoke("get_history", { 
            query: query || null, 
            limit: itemsPerPage, 
            offset: offset 
        });
        
        const rawItems = JSON.parse(jsonStr);
        const items = rawItems.map(mapApiItem);
        
        if (offset === 0 && items.length > 0) {
            lastKnownId = items[0].id;
        }
        
        clipboardItems.value = items;
        currentPageOffset.value = offset;
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to load items:", error);
    } finally {
        stopLoading();
    }
}

async function loadTotalItems() {
    try {
        const count = await invoke("db_get_count");
        totalItems.value = count;
    } catch (e) {
        console.error("Failed to load total items:", e);
        totalItems.value = 0;
    }
}

function mapApiItem(item) {
    // Map API response fields to component expected format
    const id = item.hash || item.id;
    const idx = item.offset !== undefined ? item.offset : item.index;
    // The API returns 'type', but we also check 'kind' for compatibility
    const itemType = item.type;

    return {
        id: id,
        index: idx,
        text: item.summary,
        timestamp: new Date(item.lastSeen || item.date || Date.now()).getTime(),
        byteSize: item.byteSize || item.size || 0,
        copies: item.copyCount || 0,
        firstCopied: item.timestamp ? new Date(item.timestamp).getTime() / 1000 : (new Date(item.date).getTime() / 1000),
        data: item.data,
        formats: {
            imageData: itemType === "image",
            files: (itemType === "file" || itemType === "files") ? [item.summary] : [],
        }
    };
}

// Preview logic
const previewHtml = ref("");
const previewCache = new Map();

async function fetchPreview(id) {
    if (!id) {
        previewHtml.value = "";
        return;
    }
    
    // Check cache
    if (previewCache.has(id)) {
        previewHtml.value = previewCache.get(id);
        return;
    }
    
    previewHtml.value = ""; // Clear while loading
    
    try {
        const response = await fetch(`http://localhost:3016/item/${id}/preview`);
        if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
        
        const data = await response.json();
        if (data.formatsOrder && data.formatsOrder.length > 0) {
            const preferredFormat = data.formatsOrder[0];
            const formatData = data.data[preferredFormat];
            if (formatData && formatData.html) {
                previewHtml.value = formatData.html;
                previewCache.set(id, formatData.html);
            }
        } else {
             previewHtml.value = `<div style="padding: 20px; color: var(--text-secondary); text-align: center;">No preview available</div>`;
        }
    } catch (e) {
        console.error("Failed to fetch preview:", e);
        previewHtml.value = `<div style="padding: 20px; color: var(--text-secondary); text-align: center;">Failed to load preview</div>`;
    }
}

watch(selectedItem, async (newItem) => {
    if (newItem) {
        await fetchPreview(newItem.id);
    } else {
        previewHtml.value = "";
    }
});

// Listen for toast messages from iframe
onMounted(() => {
    window.addEventListener('show-toast', (e) => {
        // TODO: Implement a proper toast notification system
        console.log("Toast:", e.detail);
    });
});

watch(searchQuery, (newQuery) => {
    selectedIndex.value = -1;
    currentPageOffset.value = 0;
    loadItems(0);
}, { debounce: 300 });

async function deleteItem(id) {
    try {
        await invoke("delete_item", { selector: id.toString() });
        clipboardItems.value = clipboardItems.value.filter(item => item.id !== id);
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

// Polling for new items
async function pollForChanges() {
    try {
        const mtimeJson = await invoke("get_mtime");
        const mtime = JSON.parse(mtimeJson);
        if (mtime.id && mtime.id !== lastKnownId && !searchQuery.value && currentPageOffset.value === 0) {
            console.log("Detected change, reloading items...");
            await loadItems(0);
            await loadTotalItems();
        }
    } catch (e) {
        // silent error
    }
}

// Shortcuts and Navigation logic
document.addEventListener("keydown", (e) => {
    handleKeyDown(e);
    if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        const key = e.key;
        
        // Handle Cmd+, to open settings (only when this window is visible/focused)
        if (key === ",") {
            e.preventDefault();
            console.log("Cmd+, pressed in ClipboardManager - opening settings");
            invoke("open_settings").catch(err => {
                console.error("Failed to open settings:", err);
            });
            return;
        }
        
        // Handle Cmd+number for quick paste
        let itemIndex = null;
        if (key >= "1" && key <= "9") itemIndex = parseInt(key) - 1;
        else if (key === "0") itemIndex = 9;
        if (itemIndex !== null && clipboardItems.value[itemIndex]) {
            e.preventDefault();
            pasteItemToSystem(clipboardItems.value[itemIndex]);
        }
    }
    if (!isCycling.value) {
        if (e.key === "ArrowDown") { e.preventDefault(); handleArrowDown(); }
        else if (e.key === "ArrowUp") { e.preventDefault(); handleArrowUp(); }
        else if (e.key === "Enter") { e.preventDefault(); handleEnter(); }
    }
});

document.addEventListener("keyup", (e) => {
    handleKeyUp(e);
    if (e.key === "Escape") {
        if (showDirModal.value) {
            localStorage.setItem('ignoreDirMismatch', 'true');
            showDirModal.value = false;
            loadItems();
            resizeWindowToFitContent();
            return;
        }
        invoke("hide_app");
    }
});

async function handleArrowDown() {
    if (clipboardItems.value.length === 0) return;
    if (selectedIndex.value === clipboardItems.value.length - 1) {
        const newOffset = currentPageOffset.value + 1;
        await loadItems(newOffset);
        if (clipboardItems.value.length > 0) {
             selectedIndex.value = clipboardItems.value.length - 1;
        } else {
             await loadItems(currentPageOffset.value - 1);
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
            const newOffset = Math.max(0, currentPageOffset.value - 1);
            await loadItems(newOffset);
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
        await invoke("hide_app");
        await invoke("paste_item", { selector: item.id.toString() });
        loadItems(currentPageOffset.value);
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

function getItemInfo(item) {
    if (!item) return null;
    if (item.formats?.imageData) return { type: "image", size: item.byteSize, label: "Image" };
    if (item.formats?.files && item.formats.files.length > 0) return { type: "files", size: `${item.formats.files.length} files`, label: "Files" };
    const wordCount = item.text ? item.text.trim().split(/\s+/).length : 0;
    return { type: "text", size: `${wordCount} words`, label: "Text" };
}

async function resizeWindowToFitContent() {
    if (!clipboardManager.value || showDirModal.value) return;
    try {
        await nextTick();
        const rect = clipboardManager.value.getBoundingClientRect();
        const contentHeight = rect.height;
        const finalHeight = Math.max(200, Math.min(600, contentHeight));
        const window = getCurrentWindow();
        await window.setSize(new LogicalSize(400, finalHeight));
    } catch (error) {
        console.error("Failed to resize window:", error);
    }
}

async function unregisterGlobalShortcut() { await invoke("unregister_main_shortcut").catch(console.error); }
async function registerGlobalShortcut() { await invoke("register_main_shortcut").catch(console.error); }

function startCyclingMode() {
    if (clipboardItems.value.length === 0) return;
    isCycling.value = true;
    selectedIndex.value = clipboardItems.value.length > 1 ? 1 : 0;
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
        } else {
            unregisterGlobalShortcut();
            if (!isCycling.value) {
                resetSelection();
                loadTotalItems();
                loadItems(0).then(() => {
                     setTimeout(() => document.querySelector(".search-input")?.focus(), 20);
                });
            }
            isCtrlPressed.value = true;
            if (!showDirModal.value) resizeWindowToFitContent();
        }
    });

    await loadTotalItems();
    await loadItems(0);
    await checkDataDirectory();

    if (clipboardManager.value) {
        resizeObserver = new ResizeObserver(() => resizeWindowToFitContent());
        resizeObserver.observe(clipboardManager.value);
    }

    pollingInterval = setInterval(pollForChanges, 1500);

    onUnmounted(() => {
        unlistenFocus();
        if (resizeObserver && clipboardManager.value) resizeObserver.disconnect();
        if (pollingInterval) clearInterval(pollingInterval);
    });
});
</script>

<template>
    <div class="clipboard-manager" ref="clipboardManager">
        <div v-if="showDirModal" class="modal-overlay">
            <div class="modal">
                <h3>Storage Location</h3>
                <p>The clipboard data directory differs from the recommended application support directory.</p>
                <div class="paths">
                    <div class="path-item"><strong>Current:</strong> {{ mismatchDirs.current }}</div>
                    <div class="path-item"><strong>Recommended:</strong> {{ mismatchDirs.expected }}</div>
                </div>
                <div class="modal-actions">
                    <button @click="handleDirChoice('create')">Create new</button>
                    <button @click="handleDirChoice('import')">Import</button>
                    <button @click="handleDirChoice('continue')" class="secondary">Continue</button>
                </div>
            </div>
        </div>

        <div class="search-container">
            <input v-model="searchQuery" type="text" :placeholder="searchPlaceholder" class="search-input" autofocus />
        </div>

        <div class="content-area">
            <div class="items-container">
                <div v-if="clipboardItems?.length === 0 && !isLoading" class="empty-state">
                    <div class="empty-icon">📋</div>
                    <p>{{ searchQuery ? "No results" : "Copy something to get started" }}</p>
                </div>
                <div v-else class="clipboard-list">
                    <ClipboardItem v-for="(item, index) in clipboardItems" :key="item.id" :item="{ ...item, index }" :selected="index === selectedIndex" @mouseenter="selectedIndex = index" @delete="deleteItem(item.id)" @select="pasteItemToSystem(item)" />
                </div>
            </div>
            
            <div class="preview-pane" v-if="selectedItem">
                <div class="preview-header">
                    <span>Preview</span>
                    <div class="preview-actions">
                        <!-- Actions like copy raw text could go here -->
                    </div>
                </div>
                <iframe v-if="previewHtml" :srcdoc="previewHtml" sandbox="allow-scripts allow-same-origin"></iframe>
                <div v-else class="loading-preview" style="display: flex; justify-content: center; align-items: center; height: 100%; color: var(--text-secondary);">
                    Loading preview...
                </div>
            </div>
        </div>

        <div v-if="showLoadingStatus || selectedItem" class="status-bar">
            <div v-if="showLoadingStatus" class="status-item loading-status">
                <div class="spinner"></div><span class="status-value">Loading...</span>
            </div>
            <template v-else-if="selectedItem">
                <div class="status-item"><span class="status-value">{{ formatFirstCopied(selectedItem.firstCopied) }}</span></div>
                <div class="status-item"><span class="status-value">{{ selectedItem.copies }}</span></div>
                <div class="status-item"><span class="status-value">{{ getItemInfo(selectedItem)?.size }}</span></div>
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
    min-height: 200px;

    .search-container { margin-top: 3px; }
    .content-area {
        display: flex;
        gap: 10px;
        flex: 1;
        min-height: 0;
    }
    .items-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0;
    }
    .preview-pane {
        width: 400px;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        
        iframe {
            flex: 1;
            border: none;
            background: white;
        }
        
        .preview-header {
            padding: 8px;
            border-bottom: 1px solid var(--border-color);
            font-size: 0.8em;
            font-weight: bold;
            display: flex;
            justify-content: space-between;
            align-items: center;
            background: var(--bg-tertiary);
        }
    }

    .clipboard-list {
        padding-top: 10px; display: flex; flex-direction: column; gap: 1px;
        .clipboard-item {
            height: 23px; overflow: hidden; cursor: default; font-size: 0.8em; display: flex; justify-content: space-between; gap: 10px; align-items: center; border-radius: 4px; padding: 1px 5px; color: var(--text-primary);
            .info { opacity: 0.6; color: var(--text-secondary); }
            &.is-selected { background: var(--accent); color: var(--accent-text); .info { color: var(--accent-text); opacity: 0.8; } }
        }
        .clipboard-item:has(img) { height: 80px; padding-top: 4px; padding-bottom: 4px; img { height: calc(100% - 8px); } }
    }
}
.modal-overlay { position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0, 0, 0, 0.5); display: flex; justify-content: center; align-items: center; z-index: 1000; padding: 20px; backdrop-filter: blur(5px); }
.modal { background: white; border-radius: 8px; padding: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); width: 100%; max-width: 400px; color: #333; h3 { margin-top: 0; } .paths { background: #f5f5f5; padding: 10px; border-radius: 4px; font-size: 0.8em; margin: 15px 0; word-break: break-all; .path-item { margin-bottom: 8px; &:last-child { margin-bottom: 0; } } } .modal-actions { display: flex; flex-direction: column; gap: 8px; button { padding: 8px; border-radius: 4px; border: none; background: var(--accent); color: white; cursor: pointer; font-weight: bold; &:hover { filter: brightness(1.1); } &.secondary { background: #e0e0e0; color: #333; } } } }
.search-input { background: var(--bg-input); border: 0.5px solid var(--border-light); border-radius: 5px; padding: 5px 8px; font-family: system-ui; box-shadow: var(--shadow-light); color: var(--text-primary); width: 100%; &::placeholder { color: var(--text-secondary); opacity: 0.7; } &:focus { outline: none; border: none; box-shadow: 0 0 0 3px var(--accent-transparent); } }
@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
.empty-state { text-align: center; color: var(--text-secondary); }
.empty-icon { font-size: 32px; filter: grayscale(0.3); }
.status-bar { display: flex; align-items: center; padding: 4px 12px; background: var(--bg-status); color: var(--text-secondary); border-radius: 4px; font-size: 0.75em; margin-top: auto; margin-bottom: 4px; flex-shrink: 0; height: 20px; line-height: 20px; }
.status-item { display: flex; align-items: center; gap: 4px; flex: 1; justify-content: center; }
.status-value { font-weight: 300; color: var(--text-secondary); }
.loading-status { justify-content: center; gap: 6px; }
.loading-status .spinner { width: 12px; height: 12px; border: 1.5px solid var(--border-color); border-radius: 50%; border-top: none; animation: spin 1s linear infinite; }
</style>
