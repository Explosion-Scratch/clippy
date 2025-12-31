import { ref, toValue } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Composable for clipboard paste/copy operations.
 * Handles both regular and plain text operations.
 * 
 * @returns {Object} Clipboard action methods
 */
export function useClipboardActions() {
    const isProcessing = ref(false);
    const error = ref(null);

    async function pasteItem(itemOrId) {
        const id = typeof itemOrId === 'object' ? itemOrId.id : toValue(itemOrId);
        if (!id) return false;
        
        isProcessing.value = true;
        error.value = null;
        
        try {
            await invoke("hide_app");
            await invoke("paste_item", { selector: id.toString() });
            return true;
        } catch (e) {
            console.error("Failed to paste item:", e);
            error.value = e;
            return false;
        } finally {
            isProcessing.value = false;
        }
    }

    async function copyItem(itemOrId) {
        const id = typeof itemOrId === 'object' ? itemOrId.id : toValue(itemOrId);
        if (!id) return false;
        
        isProcessing.value = true;
        error.value = null;
        
        try {
            await invoke("copy_item", { selector: id.toString() });
            await invoke("hide_app");
            return true;
        } catch (e) {
            console.error("Failed to copy item:", e);
            error.value = e;
            return false;
        } finally {
            isProcessing.value = false;
        }
    }

    async function getPlainText(itemOrId) {
        const item = typeof itemOrId === 'object' ? itemOrId : null;
        const id = item ? item.id : toValue(itemOrId);
        if (!id) return null;
        
        try {
            const data = await invoke("get_item_data", { id: id.toString() });
            const textPlugin = data.plugins?.find(p => p.id === 'text');
            if (textPlugin?.data) return textPlugin.data;
        } catch (e) {
            console.error("Failed to fetch item data:", e);
        }
        
        if (item?.text) return item.text;
        return null;
    }

    async function pasteAsPlainText(itemOrId) {
        const id = typeof itemOrId === 'object' ? itemOrId.id : toValue(itemOrId);
        if (!id) return false;
        
        isProcessing.value = true;
        error.value = null;
        
        try {
            await invoke("hide_app");
            await invoke("paste_item_plain_text", { id: id.toString() });
            return true;
        } catch (e) {
            console.error("Failed to paste plain text:", e);
            error.value = e;
            return false;
        } finally {
            isProcessing.value = false;
        }
    }

    async function copyAsPlainText(itemOrId) {
        const item = typeof itemOrId === 'object' ? itemOrId : null;
        const id = item ? item.id : toValue(itemOrId);
        if (!id) return false;
        
        isProcessing.value = true;
        error.value = null;
        
        try {
            const text = await getPlainText(item || id);
            if (!text) {
                error.value = new Error("No plain text available");
                return false;
            }
            
            await invoke("write_to_clipboard", { text });
            await invoke("hide_app");
            return true;
        } catch (e) {
            console.error("Failed to copy plain text:", e);
            error.value = e;
            return false;
        } finally {
            isProcessing.value = false;
        }
    }

    async function hideApp() {
        try {
            await invoke("hide_app");
            return true;
        } catch (e) {
            console.error("Failed to hide app:", e);
            return false;
        }
    }

    async function openSettings() {
        try {
            await invoke("open_settings");
            return true;
        } catch (e) {
            console.error("Failed to open settings:", e);
            error.value = e;
            return false;
        }
    }

    return {
        isProcessing,
        error,
        pasteItem,
        copyItem,
        getPlainText,
        pasteAsPlainText,
        copyAsPlainText,
        hideApp,
        openSettings
    };
}
