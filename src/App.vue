<script setup>
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const clipboardItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);

// Computed property for filtered items
const filteredItems = computed(() => {
    if (!searchQuery.value.trim()) {
        return clipboardItems.value.slice(0, 10);
    }
    return clipboardItems.value
        .filter(
            (item) =>
                item.text &&
                item.text
                    .toLowerCase()
                    .includes(searchQuery.value.toLowerCase()),
        )
        .slice(0, 10);
});

// Load recent clipboard items
async function loadRecentItems() {
    try {
        isLoading.value = true;
        const items = await invoke("db_recent_items", { count: 10, offset: 0 });
        console.log("Returned", items);
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
        // Remove from local state
        clipboardItems.value = clipboardItems.value.filter(
            (item) => item.id !== id,
        );
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

// Format timestamp for display
function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;

    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;

    return date.toLocaleDateString();
}

// Format byte size
function formatByteSize(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

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
        <header class="header">
            <h1 class="title">Clipboard History</h1>
            <div class="search-container">
                <input
                    v-model="searchQuery"
                    type="text"
                    placeholder="Search clipboard items..."
                    class="search-input"
                />
            </div>
        </header>

        <main class="main">
            <div v-if="isLoading" class="loading">
                <div class="spinner"></div>
                <p>Loading clipboard items...</p>
            </div>

            <div v-else-if="filteredItems.length === 0" class="empty-state">
                <div class="empty-icon">📋</div>
                <h3>No clipboard items found</h3>
                <p>
                    {{
                        searchQuery
                            ? "Try a different search term"
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
        </main>
    </div>
</template>

<style scoped>
.app {
    font-family:
        -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text",
        system-ui, sans-serif;
    background: #f5f5f7;
    min-height: 100vh;
    color: #1d1d1f;
}

.header {
    background: rgba(255, 255, 255, 0.8);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border-bottom: 1px solid rgba(0, 0, 0, 0.1);
    padding: 1rem 2rem;
    position: sticky;
    top: 0;
    z-index: 100;
}

.title {
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 1rem 0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.search-container {
    max-width: 600px;
    margin: 0 auto;
}

.search-input {
    width: 100%;
    padding: 0.75rem 1rem;
    border: 1px solid #d1d1d6;
    border-radius: 12px;
    font-size: 1rem;
    background: white;
    transition: all 0.2s ease;
    box-sizing: border-box;
}

.search-input:focus {
    outline: none;
    border-color: #007aff;
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.1);
}

.main {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
}

.loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    color: #86868b;
}

.spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e5e5ea;
    border-top: 3px solid #007aff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
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
    padding: 4rem 2rem;
    color: #86868b;
}

.empty-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
    opacity: 0.5;
}

.empty-state h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: #1d1d1f;
}

.empty-state p {
    margin: 0;
    font-size: 1rem;
}

.clipboard-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}
</style>
