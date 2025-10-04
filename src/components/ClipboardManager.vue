<script setup>
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { register } from "@tauri-apps/plugin-global-shortcut";
import ClipboardItem from "./ClipboardItem.vue";

const clipboardItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);
const selectedIndex = ref(-1); // -1 means no item selected
const currentPageOffset = ref(0);
const itemsPerPage = 10;

// Search for clipboard items
async function searchItems(query) {
    try {
        isLoading.value = true;
        if (!query.trim()) {
            await loadRecentItems();
            return;
        }
        const items = await invoke("db_search", { query, count: itemsPerPage });
        clipboardItems.value = items;
        currentPageOffset.value = 0;
        selectedIndex.value = -1;
    } catch (error) {
        console.error("Failed to search items:", error);
    } finally {
        isLoading.value = false;
    }
}

// Load recent clipboard items
async function loadRecentItems(offset = 0) {
    try {
        isLoading.value = true;
        const items = await invoke("db_recent_items", { count: itemsPerPage, offset });
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
    } catch (error) {
        console.error("Failed to load recent items:", error);
    } finally {
        isLoading.value = false;
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
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

// Close window on Escape key
document.addEventListener("keydown", (e) => {
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

    // Handle arrow key navigation
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
});

// Handle arrow down navigation
async function handleArrowDown() {
    if (clipboardItems.value.length === 0) return;

    // If we're at the last item, shift the window down by loading the next item
    if (selectedIndex.value === clipboardItems.value.length - 1) {
        await loadRecentItems(currentPageOffset.value + 1);
        selectedIndex.value = 9; // Keep selection at the same relative position (last item)
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
        const newOffset = Math.max(0, currentPageOffset.value - 1);
        await loadRecentItems(newOffset);
        selectedIndex.value = 0; // Keep selection at the same relative position (first item)
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

// Load items on component mount
onMounted(async () => {
    document.addEventListener("keyup", (e) => {
        console.log(e.key);
        if (e.key === "Escape") {
            let win = getCurrentWindow();
            win.hide();

            console.log({ win });
        }
    });

    // Set up focus/blur listeners to reset selection
    const unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
        console.log('Focus changed, window is focused? ' + focused);
        if (!focused) {
            // Window lost focus (blur)
            resetSelection();
            loadRecentItems();
        } else {
            // Window gained focus
            resetSelection();
            // Auto focus the search input
            document.querySelector('.search-input')?.focus();
        }
    });

    await loadRecentItems();
    // Listen for clipboard changes to refresh the list
    await listen("change-clipboard", async () => {
        resetSelection();
        await loadRecentItems();
    });

    // Clean up focus listener on unmount
    return () => {
        unlistenFocus();
    };
});
</script>

<template>
    <div class="clipboard-manager">
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
            <div v-if="isLoading" class="loading">
                <div class="spinner"></div>
            </div>

            <div v-else-if="clipboardItems?.length === 0" class="empty-state">
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
                    @delete="deleteItem(item.id)"
                />
            </div>
        </div>
    </div>
</template>

<style lang="less">
.clipboard-manager {
    --accent: lightseagreen;
    --accent-text: white;
    display: flex;
    flex-direction: column;
    font-family: system-ui, sans-serif;
    font-weight: normal;
    gap: 10px;
    padding: 8px;

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

            .info {
                opacity: 0.6;
            }

            &:hover,
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

    input {
        width: 100%;
        padding: 2px 10px;
    }
}

.loading {
    display: flex;
    justify-content: center;
    align-items: center;
}

.spinner {
    width: 20px;
    height: 20px;
    border: 2px solid;
    border-radius: 50%;
    border-top: none;
    animation: spin 1s linear infinite;
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
}

.empty-icon {
    font-size: 32px;
}

.clipboard-list {
    display: flex;
    flex-direction: column;
}
</style>