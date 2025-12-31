import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Composable for handling window focus state.
 * Self-contained with automatic lifecycle management.
 * 
 * @param {Object} options - Configuration options
 * @param {Function} [options.onFocus] - Callback when window gains focus
 * @param {Function} [options.onBlur] - Callback when window loses focus
 * @param {boolean} [options.autoRegister=true] - Whether to auto-register the listener
 * @returns {Object} Focus state and methods
 */
export function useWindowFocus(options = {}) {
    const {
        onFocus = null,
        onBlur = null,
        autoRegister = true
    } = options;
    
    const isFocused = ref(true);
    const isInitialized = ref(false);
    
    let unlisten = null;

    async function handleFocusChange(focused) {
        const wasFocused = isFocused.value;
        isFocused.value = focused;
        
        if (focused && !wasFocused && onFocus) {
            await onFocus();
        } else if (!focused && wasFocused && onBlur) {
            await onBlur();
        }
    }

    async function startListening() {
        if (unlisten) return;
        
        try {
            unlisten = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
                handleFocusChange(focused);
            });
            isInitialized.value = true;
        } catch (e) {
            console.error("Failed to register focus listener:", e);
        }
    }

    function stopListening() {
        if (unlisten) {
            unlisten();
            unlisten = null;
        }
        isInitialized.value = false;
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
        isFocused,
        isInitialized,
        startListening,
        stopListening
    };
}
