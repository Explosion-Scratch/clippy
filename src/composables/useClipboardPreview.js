import { ref, onMounted, onUnmounted, toValue } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Composable for managing clipboard preview window.
 * Handles showing, hiding, and syncing preview state.
 * 
 * @param {Object} options - Configuration options
 * @param {Ref<boolean>} [options.useInlinePreview] - Whether to use inline preview instead of external window
 * @returns {Object} Preview state and methods
 */
export function useClipboardPreview(options = {}) {
    const { useInlinePreview = ref(false) } = options;
    
    const isVisible = ref(false);
    const currentItemId = ref(null);
    const previewData = ref(null);
    const isLoading = ref(false);
    const error = ref(null);

    async function show(itemIdOrRef) {
        const itemId = toValue(itemIdOrRef);
        
        if (!itemId) {
            await hide();
            return false;
        }
        
        if (toValue(useInlinePreview)) {
            await hide();
            return false;
        }
        
        try {
            await invoke("preview_item", { id: itemId.toString() });
            currentItemId.value = itemId;
            isVisible.value = true;
            return true;
        } catch (e) {
            console.error("Failed to show preview:", e);
            error.value = e;
            return false;
        }
    }

    async function hide() {
        try {
            await invoke("hide_preview");
            isVisible.value = false;
            return true;
        } catch (e) {
            console.error("Failed to hide preview:", e);
            error.value = e;
            return false;
        }
    }

    async function focus() {
        try {
            await invoke("focus_preview");
            return true;
        } catch (e) {
            console.error("Failed to focus preview:", e);
            error.value = e;
            return false;
        }
    }

    async function checkVisibility() {
        try {
            isVisible.value = await invoke("is_preview_visible");
            return isVisible.value;
        } catch (e) {
            console.error("Failed to check preview visibility:", e);
            return false;
        }
    }

    async function fetchContent(itemIdOrRef) {
        const itemId = toValue(itemIdOrRef);
        if (!itemId) return null;
        
        isLoading.value = true;
        error.value = null;
        
        try {
            const content = await invoke("get_preview_content", { id: itemId.toString() });
            previewData.value = content;
            return content;
        } catch (e) {
            console.error("Failed to fetch preview content:", e);
            error.value = e;
            return null;
        } finally {
            isLoading.value = false;
        }
    }

    async function fetchItemData(itemIdOrRef) {
        const itemId = toValue(itemIdOrRef);
        if (!itemId) return null;
        
        try {
            const data = await invoke("get_item_data", { id: itemId.toString() });
            return data;
        } catch (e) {
            console.error("Failed to fetch item data:", e);
            error.value = e;
            return null;
        }
    }

    async function openInDashboard(itemIdOrRef) {
        const itemId = toValue(itemIdOrRef);
        if (!itemId) return false;
        
        try {
            await invoke("open_in_dashboard", { id: itemId.toString() });
            return true;
        } catch (e) {
            console.error("Failed to open in dashboard:", e);
            error.value = e;
            return false;
        }
    }

    return {
        isVisible,
        currentItemId,
        previewData,
        isLoading,
        error,
        show,
        hide,
        focus,
        checkVisibility,
        fetchContent,
        fetchItemData,
        openInDashboard
    };
}
