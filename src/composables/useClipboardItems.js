import { ref, computed, toValue } from "vue";
import { invoke } from "@tauri-apps/api/core";

const PAGE_SIZE = 30;
const API_FAILURE_THRESHOLD = 3;

/**
 * Composable for managing clipboard items - loading, pagination, and CRUD operations.
 * Fully self-contained with its own state management.
 * 
 * @returns {Object} Clipboard items state and methods
 */
export function useClipboardItems() {
    const allLoadedItems = ref([]);
    const totalItems = ref(0);
    const isLoading = ref(false);
    const isLoadingMore = ref(false);
    const searchQuery = ref("");
    const apiStatus = ref('connected');
    const error = ref(null);
    
    let consecutiveApiFailures = 0;

    function mapApiItem(item) {
        const id = item.hash || item.id;
        const idx = item.offset !== undefined ? item.offset : item.index;
        const itemType = item.type;

        return {
            id,
            index: idx,
            text: item.summary,
            timestamp: new Date(item.lastSeen || item.date || Date.now()).getTime(),
            byteSize: item.byteSize || item.size || 0,
            copies: item.copyCount || 0,
            firstCopied: item.timestamp 
                ? new Date(item.timestamp).getTime() / 1000 
                : new Date(item.date).getTime() / 1000,
            data: item.data,
            formats: {
                imageData: itemType === "image",
                files: ['file', 'files'].includes(itemType) ? [item.summary] : [],
            }
        };
    }

    async function loadItems(options = {}) {
        const { append = false, preserveId = null, query = null } = options;
        const searchTerm = query !== null ? toValue(query) : toValue(searchQuery);
        
        if (!append) {
            isLoading.value = true;
            error.value = null;
        }
        isLoadingMore.value = append;
        
        try {
            const offset = append ? allLoadedItems.value.length : 0;
            const result = await invoke("get_history", {
                limit: PAGE_SIZE,
                offset,
                query: searchTerm || null,
                sort: "lastSeen",
                order: "desc"
            });
            
            consecutiveApiFailures = 0;
            if (apiStatus.value === 'error') {
                apiStatus.value = 'connected';
            }
            
            const parsed = JSON.parse(result);
            const mapped = parsed.map(mapApiItem);
            
            if (append) {
                allLoadedItems.value = [...allLoadedItems.value, ...mapped];
            } else {
                allLoadedItems.value = mapped;
            }
            
            const preserveIdValue = toValue(preserveId);
            if (preserveIdValue) {
                return allLoadedItems.value.findIndex(item => item.id === preserveIdValue);
            }
            return -1;
        } catch (e) {
            console.error("Failed to load clipboard items:", e);
            error.value = e;
            consecutiveApiFailures++;
            if (consecutiveApiFailures >= API_FAILURE_THRESHOLD && apiStatus.value !== 'dismissed') {
                apiStatus.value = 'error';
            }
            return -1;
        } finally {
            isLoading.value = false;
            isLoadingMore.value = false;
        }
    }

    async function loadMore() {
        if (isLoadingMore.value || allLoadedItems.value.length >= totalItems.value) {
            return false;
        }
        await loadItems({ append: true });
        return true;
    }

    async function loadTotal() {
        try {
            const count = await invoke("db_get_count");
            totalItems.value = count;
            return count;
        } catch (e) {
            console.error("Failed to load total items:", e);
            totalItems.value = 0;
            return 0;
        }
    }

    async function deleteItem(idOrRef) {
        const id = toValue(idOrRef);
        try {
            await invoke("delete_item", { selector: id.toString() });
            allLoadedItems.value = allLoadedItems.value.filter(item => item.id !== id);
            return true;
        } catch (e) {
            console.error("Failed to delete item:", e);
            error.value = e;
            return false;
        }
    }

    async function restartApi() {
        try {
            await invoke("restart_api");
            apiStatus.value = 'connected';
            consecutiveApiFailures = 0;
            await loadItems();
            return true;
        } catch (e) {
            console.error("Failed to restart API:", e);
            error.value = e;
            return false;
        }
    }

    function dismissApiError() {
        apiStatus.value = 'dismissed';
    }

    function reset() {
        allLoadedItems.value = [];
        totalItems.value = 0;
        searchQuery.value = "";
        apiStatus.value = 'connected';
        error.value = null;
        consecutiveApiFailures = 0;
    }

    const items = computed(() => allLoadedItems.value);
    const hasMore = computed(() => allLoadedItems.value.length < totalItems.value);
    const isEmpty = computed(() => allLoadedItems.value.length === 0 && !isLoading.value);

    return {
        items,
        allLoadedItems,
        totalItems,
        isLoading,
        isLoadingMore,
        searchQuery,
        apiStatus,
        error,
        hasMore,
        isEmpty,
        loadItems,
        loadMore,
        loadTotal,
        deleteItem,
        restartApi,
        dismissApiError,
        reset
    };
}
