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
let resizeObserver = null;

// Cycling mode state
const isCycling = ref(false);
const isCtrlPressed = ref(true);

// Get the currently selected item
const selectedItem = computed(() => {
    console.log({
        selectedIndex: selectedIndex.value,
        clipboardItems: clipboardItems.value,
    });
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        return clipboardItems.value[selectedIndex.value];
    }
    return null;
});

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
        const items = await invoke("db_search", { query, count: itemsPerPage });
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

// Load recent clipboard items
async function loadRecentItems(offset = 0) {
    try {
        startLoading();
        const items = await invoke("db_recent_items", {
            count: itemsPerPage,
            offset,
        });
        console.log("=== RECEIVED ITEMS FROM BACKEND ===");
        console.log("Raw items:", items);
        items.forEach((item, index) => {
            console.log(`Item ${index}:`, {
                id: item.id,
                timestamp: item.timestamp,
                timestampType: typeof item.timestamp,
                byteSize: item.byteSize,
                byteSizeType: typeof item.byteSize,
                text: item.text,
            });
        });
        console.log("====================================");
        clipboardItems.value = items;
        currentPageOffset.value = offset;
        selectedIndex.value = -1;

        // Resize window after content loads
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to load recent items:", error);
    } finally {
        stopLoading();
    }
}

// Watch search query and trigger search
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
        await invoke("db_delete_item", { id });
        clipboardItems.value = clipboardItems.value.filter(
            (item) => item.id !== id,
        );
        // Resize window after deletion
        await resizeWindowToFitContent();
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

// Close window on Escape key
document.addEventListener("keydown", (e) => {
    // Handle cycling detection
    handleKeyDown(e);

    // Handle Command + number keys for system paste
    if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        const key = e.key;
        let itemIndex = null;

        // Map 1-9 keys to indices 0-8, and 0 key to index 9
        if (key >= "1" && key <= "9") {
            itemIndex = parseInt(key) - 1;
        } else if (key === "0") {
            itemIndex = 9;
        }

        // If we have a valid item index and the item exists
        if (itemIndex !== null && clipboardItems.value[itemIndex]) {
            e.preventDefault();
            pasteItemToSystem(clipboardItems.value[itemIndex]);
        }
    }

    // Handle arrow key navigation (only when not in cycling mode)
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

// Handle key up events for Ctrl release detection
document.addEventListener("keyup", (e) => {
    handleKeyUp(e);

    console.log(e.key);
    if (e.key === "Escape") {
        let win = getCurrentWindow();
        win.hide();

        // Re-register global shortcut when window is hidden
        registerGlobalShortcut();

        console.log({ win });
    }
});

// Handle arrow down navigation
async function handleArrowDown() {
    if (clipboardItems.value.length === 0) return;

    // If we're at the last item, shift the window down by loading the next item
    if (selectedIndex.value === clipboardItems.value.length - 1) {
        const previousOffset = currentPageOffset.value;
        const previousItems = [...clipboardItems.value];
        const previousSelectedIndex = selectedIndex.value;

        await loadRecentItems(currentPageOffset.value + 1);

        // Check if new items were loaded, if not, revert changes
        if (clipboardItems.value.length === 0) {
            currentPageOffset.value = previousOffset;
            clipboardItems.value = previousItems;
            selectedIndex.value = previousSelectedIndex;
            return;
        }

        // Keep selection at the last valid position
        selectedIndex.value = Math.min(9, clipboardItems.value.length - 1);
    } else {
        // Move to next item on current window
        selectedIndex.value = selectedIndex.value + 1;
    }
}

// Handle arrow up navigation
async function handleArrowUp() {
    if (clipboardItems.value.length === 0) return;

    // If we're at the first item and not at the very beginning, shift the window up
    if (selectedIndex.value === 0 && currentPageOffset.value > 0) {
        const previousOffset = currentPageOffset.value;
        const previousItems = [...clipboardItems.value];
        const previousSelectedIndex = selectedIndex.value;

        const newOffset = Math.max(0, currentPageOffset.value - 1);
        await loadRecentItems(newOffset);

        // Check if new items were loaded, if not, revert changes
        if (clipboardItems.value.length === 0) {
            currentPageOffset.value = previousOffset;
            clipboardItems.value = previousItems;
            selectedIndex.value = previousSelectedIndex;
            return;
        }

        // Keep selection at the first valid position
        selectedIndex.value = 0;
    } else {
        // Move to previous item on current window
        selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    }
}

// Handle enter key to select and inject item
function handleEnter() {
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        pasteItemToSystem(clipboardItems.value[selectedIndex.value]);
    }
}

// System paste function
async function pasteItemToSystem(item) {
    try {
        console.log("Injecting item from ID:", item.id);
        const result = await invoke("inject_item", { id: item.id });
        console.log("Item injection result:", result);

        // Reload the items to show updated copies count
        await loadRecentItems(currentPageOffset.value);
    } catch (error) {
        console.error("Failed to inject item:", error);
    }
}

// Reset selection index
function resetSelection() {
    selectedIndex.value = -1;
    searchQuery.value = "";
    currentPageOffset.value = 0;
}

// Format first copied timestamp for display
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

// Format byte size to be more compact
function formatByteSize(bytes) {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}K`;
    return `${Math.round(bytes / (1024 * 1024))}M`;
}

// Count words in text
function countWords(text) {
    if (!text) return 0;
    return text
        .trim()
        .split(/\s+/)
        .filter((word) => word.length > 0).length;
}

// Get content type and size info for selected item
function getItemInfo(item) {
    if (!item) return null;

    if (item.formats?.imageData) {
        return {
            type: "image",
            size: formatByteSize(item.byteSize),
            label: "Image",
        };
    }

    if (item.formats?.files && item.formats.files.length > 0) {
        return {
            type: "files",
            size: `${item.formats.files.length} file${item.formats.files.length > 1 ? "s" : ""}`,
            label: "Files",
        };
    }

    // Text content
    const wordCount = countWords(item.text || "");
    return {
        type: "text",
        size: `${wordCount} words`,
        label: "Text",
    };
}

// Resize window to fit content
async function resizeWindowToFitContent() {
    if (!clipboardManager.value) return;

    try {
        // Wait for next tick to ensure DOM is updated
        await nextTick();

        // Get the actual height of the content
        const rect = clipboardManager.value.getBoundingClientRect();
        const contentHeight = rect.height;

        // Set reasonable bounds
        const minHeight = 200;
        const maxHeight = 600;
        const finalHeight = Math.max(
            minHeight,
            Math.min(maxHeight, contentHeight),
        );

        // Set a fixed width (can be adjusted based on content)
        const width = 400;

        // Resize the window
        const window = getCurrentWindow();
        await window.setSize(new LogicalSize(width, finalHeight));

        console.log(
            `Window resized to ${width}x${finalHeight}px (content was ${contentHeight}px)`,
        );
    } catch (error) {
        console.error("Failed to resize window:", error);
    }
}

// Unregister global shortcut when window opens
async function unregisterGlobalShortcut() {
    try {
        await invoke("unregister_main_shortcut");
        console.log("Global shortcut unregistered");
    } catch (error) {
        console.error("Failed to unregister global shortcut:", error);
    }
}

// Register global shortcut when window closes
async function registerGlobalShortcut() {
    try {
        await invoke("register_main_shortcut");
        console.log("Global shortcut registered");
    } catch (error) {
        console.error("Failed to register global shortcut:", error);
    }
}

// Start cycling mode (called when Ctrl+P is held)
function startCyclingMode() {
    console.log("Starting cycling mode");
    isCycling.value = true;

    // Start with first item selected
    if (clipboardItems.value.length > 0) {
        selectedIndex.value = 0;
    }
}

// Handle cycling to next item
function cycleToNext() {
    if (!isCycling.value || clipboardItems.value.length === 0) return;

    console.log("Cycling to next item");

    // Move to next item, wrap around if at end
    selectedIndex.value =
        (selectedIndex.value + 1) % clipboardItems.value.length;
}

// Handle end cycling (paste selected item)
async function endCycling() {
    if (!isCycling.value) return;

    console.log("Ending cycling mode");
    isCycling.value = false;

    // If we have a selected item, paste it
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        await pasteItemToSystem(clipboardItems.value[selectedIndex.value]);
    }

    // Re-register global shortcut before closing window
    await registerGlobalShortcut();

    // Close window after pasting
    const window = getCurrentWindow();
    window.hide();
}

// Handle keyboard events for cycling detection
function handleKeyDown(e) {
    // Track Ctrl key state
    if (e.key === "Control" && !isCtrlPressed.value) {
        isCtrlPressed.value = true;
        console.log("Ctrl key pressed");
    }

    console.log({
        isCtrlPressed: isCtrlPressed.value,
        key: e.key,
        isCycling: isCycling.value,
    });

    // If Ctrl is pressed and P is pressed, cycle
    if (isCtrlPressed.value && e.key === "p") {
        e.preventDefault();
        console.log("P pressed while Ctrl held - cycling");

        if (!isCycling.value) {
            startCyclingMode();
        } else {
            cycleToNext();
        }
    }
}

function handleKeyUp(e) {
    // When Ctrl is released, end cycling if active
    if (e.key === "Control") {
        console.log("Ctrl key released");
        isCtrlPressed.value = false;

        // End cycling and paste if we were cycling
        if (isCycling.value) {
            endCycling();
        }
    }
}

// Load items on component mount
onMounted(async () => {
    // Set up focus/blur listeners to reset selection
    const unlistenFocus = await getCurrentWindow().onFocusChanged(
        ({ payload: focused }) => {
            console.log("Focus changed, window is focused? " + focused);
            if (!focused) {
                // Window lost focus (blur)
                // End cycling mode if active
                if (isCycling.value) {
                    isCycling.value = false;
                    isCtrlPressed.value = false;
                }
                // Re-register global shortcut when window loses focus
                registerGlobalShortcut();
                resetSelection();
                loadRecentItems();
            } else {
                // Window gained focus
                // Unregister global shortcut so frontend can handle keyboard events
                unregisterGlobalShortcut();
                if (!isCycling.value) {
                    resetSelection();
                    // Auto focus the search input
                    document.querySelector(".search-input")?.focus();
                }
                isCtrlPressed.value = true;
                // Resize window when gaining focus
                resizeWindowToFitContent();
            }
        },
    );

    await loadRecentItems();

    // Set up ResizeObserver to monitor content size changes
    if (clipboardManager.value) {
        resizeObserver = new ResizeObserver(() => {
            resizeWindowToFitContent();
        });
        resizeObserver.observe(clipboardManager.value);
    }

    // Listen for clipboard changes to refresh the list
    await listen("change-clipboard", async () => {
        resetSelection();
        await loadRecentItems();
    });

    // Clean up focus listener on unmount
    return () => {
        unlistenFocus();
        if (resizeObserver && clipboardManager.value) {
            resizeObserver.disconnect();
        }
        // Re-register global shortcut when component unmounts
        registerGlobalShortcut();
    };
});
</script>

<template>
    <div class="clipboard-manager" ref="clipboardManager">
        <!-- Search bar -->
        <div class="search-container">
            <input
                v-model="searchQuery"
                type="text"
                placeholder="Search..."
                class="search-input"
                autofocus
            />
        </div>

        <!-- Clipboard items list -->
        <div class="items-container">
            <div v-if="clipboardItems?.length === 0 && !isLoading" class="empty-state">
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
            <!-- Loading status -->
            <div v-if="showLoadingStatus" class="status-item loading-status">
                <div class="spinner"></div>
                <span class="status-value">Loading...</span>
            </div>
            
            <!-- Selected item info -->
            <template v-else-if="selectedItem">
                <div class="status-item">
                    <svg
                        class="status-icon"
                        xmlns="http://www.w3.org/2000/svg"
                        width="32"
                        height="32"
                        viewBox="0 0 256 256"
                    >
                        <path
                            fill="currentColor"
                            d="M208 32h-24v-8a8 8 0 0 0-16 0v8H88v-8a8 8 0 0 0-16 0v8H48a16 16 0 0 0-16 16v160a16 16 0 0 0 16 16h160a16 16 0 0 0 16-16V48a16 16 0 0 0-16-16M72 48v8a8 8 0 0 0 16 0v-8h80v8a8 8 0 0 0 16 0v-8h24v32H48V48Zm136 160H48V96h160zm-96-88v64a8 8 0 0 1-16 0v-51.06l-4.42 2.22a8 8 0 0 1-7.16-14.32l16-8A8 8 0 0 1 112 120m59.16 30.45L152 176h16a8 8 0 0 1 0 16h-32a8 8 0 0 1-6.4-12.8l28.78-38.37a8 8 0 1 0-13.31-8.83a8 8 0 1 1-13.85-8A24 24 0 0 1 176 136a23.76 23.76 0 0 1-4.84 14.45"
                        />
                    </svg>
                    <span class="status-value">{{
                        formatFirstCopied(selectedItem.firstCopied)
                    }}</span>
                </div>
                <div class="status-item">
                    <svg
                        class="status-icon"
                        xmlns="http://www.w3.org/2000/svg"
                        width="32"
                        height="32"
                        viewBox="0 0 256 256"
                    >
                        <path
                            fill="currentColor"
                            d="M216 152h-48v-48h48a8 8 0 0 0 0-16h-48V40a8 8 0 0 0-16 0v48h-48V40a8 8 0 0 0-16 0v48H40a8 8 0 0 0 0 16h48v48H40a8 8 0 0 0 0 16h48v48a8 8 0 0 0 16 0v-48h48v48a8 8 0 0 0 16 0v-48h48a8 8 0 0 0 0-16m-112 0v-48h48v48Z"
                        />
                    </svg>
                    <span class="status-value">{{ selectedItem.copies }}</span>
                </div>
                <div class="status-item">
                    <svg
                        class="status-icon"
                        xmlns="http://www.w3.org/2000/svg"
                        width="32"
                        height="32"
                        viewBox="0 0 256 256"
                    >
                        <path
                            fill="currentColor"
                            d="M128 24c-53.83 0-96 24.6-96 56v96c0 31.4 42.17 56 96 56s96-24.6 96-56V80c0-31.4-42.17-56-96-56m80 104c0 9.62-7.88 19.43-21.61 26.92C170.93 163.35 150.19 168 128 168s-42.93-4.65-58.39-13.08C55.88 147.43 48 137.62 48 128v-16.64c17.06 15 46.23 24.64 80 24.64s62.94-9.68 80-24.64ZM69.61 53.08C85.07 44.65 105.81 40 128 40s42.93 4.65 58.39 13.08C200.12 60.57 208 70.38 208 80s-7.88 19.43-21.61 26.92C170.93 115.35 150.19 120 128 120s-42.93-4.65-58.39-13.08C55.88 99.43 48 89.62 48 80s7.88-19.43 21.61-26.92m116.78 149.84C170.93 211.35 150.19 216 128 216s-42.93-4.65-58.39-13.08C55.88 195.43 48 185.62 48 176v-16.64c17.06 15 46.23 24.64 80 24.64s62.94-9.68 80-24.64V176c0 9.62-7.88 19.43-21.61 26.92"
                        />
                    </svg>
                    <span class="status-value">{{
                        getItemInfo(selectedItem)?.size
                    }}</span>
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



.search-input {
    background: var(--bg-input);
    border: 0.5px solid var(--border-light);
    border-radius: 5px;
    padding: 3px 6px;
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
        border-color: var(--accent);
        box-shadow: 0 0 0 3px var(--accent-transparent);
    }
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }
    100% {
        transform: rotate(360deg);
    }
}

.empty-state {
    text-align: center;
    color: var(--text-secondary);
}

.empty-icon {
    font-size: 32px;
    filter: grayscale(0.3);
}

.clipboard-list {
    display: flex;
    flex-direction: column;
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
    opacity: 0.8;
    border: 1px solid var(--border-color);
}

.status-item {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    justify-content: center;
}

.status-icon {
    width: 14px;
    height: 14px;
    opacity: 0.7;
    color: var(--text-secondary);
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
