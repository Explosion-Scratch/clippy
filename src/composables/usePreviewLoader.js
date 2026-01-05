import { ref, toValue } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Composable for loading preview content with fast text-first loading.
 * Handles abort logic for switching items and proper request coordination.
 * 
 * @returns {Object} Preview loading state and methods
 */
export function usePreviewLoader() {
    const isLoading = ref(false);
    const loadingText = ref("");
    const previewData = ref(null);
    const error = ref(null);
    const currentItemId = ref(null);
    
    let textAbortController = null;
    let previewAbortController = null;
    let currentRequestId = 0;

    function abortAllRequests() {
        if (textAbortController) {
            textAbortController.abort();
            textAbortController = null;
        }
        if (previewAbortController) {
            previewAbortController.abort();
            previewAbortController = null;
        }
    }

    function reset() {
        abortAllRequests();
        isLoading.value = false;
        loadingText.value = "";
        previewData.value = null;
        error.value = null;
        currentItemId.value = null;
    }

    async function load(itemIdOrRef) {
        const itemId = toValue(itemIdOrRef);
        
        abortAllRequests();
        
        if (!itemId) {
            reset();
            return null;
        }

        const requestId = ++currentRequestId;
        currentItemId.value = itemId;
        isLoading.value = true;
        loadingText.value = "";
        previewData.value = null;
        error.value = null;

        textAbortController = new AbortController();
        previewAbortController = new AbortController();
        const textSignal = textAbortController.signal;
        const previewSignal = previewAbortController.signal;

        const textPromise = invoke("get_item_text", { id: itemId })
            .then(data => {
                if (textSignal.aborted || requestId !== currentRequestId) return;
                if (data?.text && isLoading.value) {
                    loadingText.value = data.text;
                }
            })
            .catch(() => {});

        const previewPromise = invoke("get_preview_content", { id: itemId })
            .then(data => {
                if (previewSignal.aborted || requestId !== currentRequestId) return null;
                
                if (textAbortController) {
                    textAbortController.abort();
                    textAbortController = null;
                }
                
                return data;
            })
            .catch(e => {
                if (previewSignal.aborted || requestId !== currentRequestId) return null;
                console.error("Failed to fetch preview:", e);
                error.value = "Failed to load preview";
                return null;
            });

        try {
            const data = await previewPromise;
            
            if (requestId !== currentRequestId) return null;
            
            if (data) {
                previewData.value = data;
                loadingText.value = "";
            }
            
            return data;
        } finally {
            if (requestId === currentRequestId) {
                isLoading.value = false;
            }
        }
    }

    return {
        isLoading,
        loadingText,
        previewData,
        error,
        currentItemId,
        load,
        reset,
        abortAllRequests
    };
}
