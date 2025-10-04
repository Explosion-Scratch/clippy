<script setup>
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { register } from "@tauri-apps/plugin-global-shortcut";
import ClipboardItem from "./components/ClipboardItem.vue";

const clipboardItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);

// Search for clipboard items
async function searchItems(query) {
    try {
        isLoading.value = true;
        if (!query.trim()) {
            await loadRecentItems();
            return;
        }
        const items = await invoke("db_search", { query, count: 20 });
        clipboardItems.value = items;
    } catch (error) {
        console.error("Failed to search items:", error);
    } finally {
        isLoading.value = false;
    }
}

// Load recent clipboard items
async function loadRecentItems() {
    try {
        isLoading.value = true;
        const items = await invoke("db_recent_items", { count: 20, offset: 0 });
        console.log("=== RECEIVED ITEMS FROM BACKEND ===");
        console.log("Raw items:", items);
        items.forEach((item, index) => {
            console.log(`Item ${index}:`, {
                id: item.id,
                timestamp: item.timestamp,
                timestampType: typeof item.timestamp,
                byteSize: item.byte_size,
                byteSizeType: typeof item.byte_size,
                text: item.text,
            });
        });
        console.log("====================================");
        clipboardItems.value = items;
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
});

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

    await loadRecentItems();
    // Listen for clipboard changes to refresh the list
    await listen("change-clipboard", async () => {
        await loadRecentItems();
    });
});
</script>

<template>
    <div class="app">
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
                    @delete="deleteItem(item.id)"
                />
            </div>
        </div>
    </div>
</template>

<style lang="less">
html,
body {
    background: transparent !important;
    padding: 0 !important;
    margin: 0 !important;
}

* {
    box-sizing: border-box;
}

html,
body,
#app {
    overflow: hidden;
}

#app {
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

            &:hover {
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
