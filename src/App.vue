<script setup>
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ClipboardItem from "./components/ClipboardItem.vue";

const clipboardItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);

// Computed property for filtered items
const filteredItems = computed(() => {
    if (!searchQuery.value.trim()) {
        return clipboardItems.value.slice(0, 20);
    }
    return clipboardItems.value
        .filter(
            (item) =>
                item.text &&
                item.text
                    .toLowerCase()
                    .includes(searchQuery.value.toLowerCase()),
        )
        .slice(0, 20);
});

// Load recent clipboard items
async function loadRecentItems() {
    try {
        isLoading.value = true;
        const items = await invoke("db_recent_items", { count: 20, offset: 0 });
        clipboardItems.value = items;
    } catch (error) {
        console.error("Failed to load recent items:", error);
    } finally {
        isLoading.value = false;
    }
}

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
    if (e.key === "Escape") {
        window.__TAURI__.window.getCurrent().close();
    }
});

// Load items on component mount
onMounted(async () => {
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

            <div v-else-if="filteredItems?.length === 0" class="empty-state">
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
                    v-for="item in filteredItems"
                    :key="item.id"
                    :item="item"
                    @delete="deleteItem(item.id)"
                />
            </div>
        </div>
    </div>
</template>

<style scoped>
.app {
    font-family:
        -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
    background: rgba(246, 246, 246, 0.95);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    width: 300px;
    height: 600px;
    border-radius: 12px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: #1d1d1f;
}

.search-container {
    padding: 12px;
    background: rgba(255, 255, 255, 0.7);
    border-bottom: 1px solid rgba(0, 0, 0, 0.1);
}

.search-input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 8px;
    font-size: 14px;
    background: rgba(255, 255, 255, 0.8);
    transition: all 0.2s ease;
    box-sizing: border-box;
}

.search-input:focus {
    outline: none;
    border-color: #007aff;
    box-shadow: 0 0 0 2px rgba(0, 122, 255, 0.2);
    background: rgba(255, 255, 255, 0.95);
}

.search-input::placeholder {
    color: #8e8e93;
}

.items-container {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
}

.items-container::-webkit-scrollbar {
    width: 4px;
}

.items-container::-webkit-scrollbar-track {
    background: transparent;
}

.items-container::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 2px;
}

.items-container::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.3);
}

.loading {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 40px 20px;
}

.spinner {
    width: 20px;
    height: 20px;
    border: 2px solid rgba(0, 0, 0, 0.1);
    border-top: 2px solid #007aff;
    border-radius: 50%;
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
    padding: 40px 20px;
    color: #8e8e93;
}

.empty-icon {
    font-size: 32px;
    margin-bottom: 8px;
    opacity: 0.6;
}

.empty-state p {
    margin: 0;
    font-size: 13px;
    font-weight: 400;
}

.clipboard-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
</style>
