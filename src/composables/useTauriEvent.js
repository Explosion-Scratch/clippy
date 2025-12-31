import { ref, onMounted, onUnmounted, toValue } from "vue";
import { listen } from "@tauri-apps/api/event";

/**
 * Composable for listening to Tauri events.
 * Fully self-contained with automatic cleanup.
 * 
 * @param {string} eventName - Name of the Tauri event to listen for
 * @param {Function} handler - Event handler function
 * @param {Object} options - Configuration options
 * @param {boolean} [options.autoRegister=true] - Whether to auto-register on mount
 * @returns {Object} Event listener state and methods
 */
export function useTauriEvent(eventName, handler, options = {}) {
    const { autoRegister = true } = options;
    
    const isListening = ref(false);
    let unlisten = null;

    async function startListening() {
        if (unlisten) return;
        
        try {
            unlisten = await listen(toValue(eventName), (event) => {
                handler(event.payload, event);
            });
            isListening.value = true;
        } catch (e) {
            console.error(`Failed to listen to event ${eventName}:`, e);
        }
    }

    function stopListening() {
        if (unlisten) {
            unlisten();
            unlisten = null;
        }
        isListening.value = false;
    }

    if (autoRegister) {
        onMounted(() => {
            startListening();
        });

        onUnmounted(() => {
            stopListening();
        });
    }

    return {
        isListening,
        startListening,
        stopListening
    };
}

/**
 * Composable for listening to clipboard change events.
 * Specialized version of useTauriEvent for clipboard-changed events.
 * 
 * @param {Function} handler - Handler called with new clipboard item ID
 * @param {Object} options - Configuration options
 * @returns {Object} Event listener state and methods
 */
export function useClipboardChangeEvent(handler, options = {}) {
    return useTauriEvent("clipboard-changed", handler, options);
}

/**
 * Composable for listening to preview item change events.
 * 
 * @param {Function} handler - Handler called with new item ID
 * @param {Object} options - Configuration options
 * @returns {Object} Event listener state and methods
 */
export function usePreviewChangeEvent(handler, options = {}) {
    return useTauriEvent("preview-item-changed", handler, options);
}
