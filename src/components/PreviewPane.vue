<script setup>
import { ref, watch, nextTick, onUnmounted, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { showToast } from "../utils/ui";
import PreviewFooter from "./PreviewFooter.vue";

const EDIT_SAVE_TIMEOUT_MS = 10000;

const props = defineProps({
    itemId: {
        type: String,
        default: null
    },
    keyboardState: {
        type: Object,
        default: () => ({ currentlyPressed: [], itemShortcuts: [] })
    },
    isInline: {
        type: Boolean,
        default: false
    }
});

const emit = defineEmits(["refresh"]);

import { usePreviewLoader } from "../composables";

// ... (props and emits remain)

const { 
    isLoading, 
    loadingText, 
    previewData, 
    error: loaderError, 
    currentItemId,
    load, 
    reset: resetLoader 
} = usePreviewLoader();

const previewContent = ref("");
// isLoading and loadingText are now from composable
const error = ref(null); // Local error for edit/save actions, separate from loader error
const isEditing = ref(false);
const editedText = ref("");
const originalText = ref("");
const plainTextContent = ref("");
const isEditable = ref(true);
const itemKind = ref("text");
const frameRef = ref(null);
const isSaving = ref(false);
const editTextareaRef = ref(null);

const activeError = computed(() => error.value || loaderError.value);

const isAltPressed = computed(() => {
    const pressed = props.keyboardState.currentlyPressed || [];
    return pressed.includes('Alt');
});

let frameDblHandler = null;
let messageHandler = null;

function resetState() {
    resetLoader();
    previewContent.value = "";
    error.value = null;
    isEditing.value = false;
    editedText.value = "";
    originalText.value = "";
    plainTextContent.value = "";
    itemKind.value = "text";
    isEditable.value = true;
    isSaving.value = false;
}

// ... (startEdit, cancelEdit remain same)

async function saveEdit() {
    if (!currentItemId.value || isSaving.value) return;
    
    isSaving.value = true;
    error.value = null;
    
    try {
        const responseJson = await invoke("edit_item", { 
            id: currentItemId.value, 
            formats: { text: editedText.value } 
        });
        const newItem = JSON.parse(responseJson);
        const newId = newItem.hash || newItem.id || currentItemId.value;
        
        isEditing.value = false;
        originalText.value = editedText.value;
        
        emit("refresh", newId);
        await loadPreview(newId);
    } catch (e) {
        console.error("Failed to save edit:", e);
        error.value = "Failed to save changes. Please try again.";
    } finally {
        isSaving.value = false;
    }
}

// abortPreviousRequest removed (handled by composable)

async function loadPreview(id) {
    if (!id) {
        resetState();
        return;
    }

    // Reset local view state before loading (but composable handles its own state)
    error.value = null;
    isEditing.value = false;
    previewContent.value = "";
    
    // Load data using composable
    // The composable handles fast text loading and aborts automatically
    const data = await load(id);

    if (!data) return; // Aborted or failed

    try {
        itemKind.value = data.kind || "text";
        isEditable.value = itemKind.value !== "file" && itemKind.value !== "image";

        const formatsOrder = data.formatsOrder || [];
        const dataMap = data.data || {};

        let html = "";
        let text = "";
        let pureText = "";
        for (const formatId of formatsOrder) {
            const formatData = dataMap[formatId];
            if (!formatData) continue;
            if (!html && formatData.html) html = formatData.html;
            if (!text && formatData.text) text = formatData.text;
            if (!pureText && formatId === "text" && formatData.text) {
                pureText = formatData.text;
            }
        }

        if (html) {
            html = sanitizePreviewHtml(html, id);
        }

        // Check if we became stale during processing (unlikely with await but good safety)
        if (currentItemId.value !== id) return;

        previewContent.value = html || "<div class='empty'>No preview available</div>";
        plainTextContent.value = pureText || text || "";
        originalText.value = plainTextContent.value;
        editedText.value = plainTextContent.value;

        await nextTick();
    } catch (e) {
        console.error("Failed to process preview:", e);
        error.value = "Failed to process preview";
    }
}

function sanitizePreviewHtml(html, id) {
    html = html.replace('<html>', '<html class="compact">');
    
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#20b2aa';
    const accentTransparent = getComputedStyle(document.documentElement).getPropertyValue('--accent-transparent').trim() || 'rgba(32, 178, 170, 0.25)';
    
    const accentStyle = `<style>:root { --accent: ${accent}; --accent-transparent: ${accentTransparent}; }</style>`;
    
    const dblScript = `<` + 'script' + `>
document.addEventListener('dblclick', (e) => {
    e.preventDefault();
    try { parent.postMessage({ type: 'preview-dblclick', id: '${id}' }, '*'); } catch (_) {}
});
</` + 'script' + `>`;
    
    if (html.includes("</head>")) {
        html = html.replace("</head>", `${accentStyle}</head>`);
    } else if (html.includes("<body")) {
        html = html.replace("<body", `${accentStyle}<body`);
    } else {
        html = `${accentStyle}${html}`;
    }
    
    html = html.includes("</body>") 
        ? html.replace("</body>", `${dblScript}</body>`) 
        : `${html}${dblScript}`;
    
    return html;
}

watch(() => props.itemId, (id) => {
    loadPreview(id);
}, { immediate: true });

function attachFrameDblclick() {
    if (!frameRef.value) return;
    
    const doc = frameRef.value.contentDocument || frameRef.value.contentWindow?.document;
    if (!doc || !doc.body) return;
    
    if (frameDblHandler) {
        doc.removeEventListener("dblclick", frameDblHandler);
    }
    
    frameDblHandler = (event) => {
        event.preventDefault();
        event.stopPropagation();
        startEdit();
    };
    doc.addEventListener("dblclick", frameDblHandler);
}

function handleFrameLoad() {
    attachFrameDblclick();
}

function handlePostMessage(event) {
    if (!event?.data) return;
    
    // Handle specific message types
    if (event.data.type === "copy") {
        if (event.data.text) {
             // Use the Tauri clipboard API via backend or navigator if available
             // Since we are in Tauri, navigator.clipboard should work for text
             navigator.clipboard.writeText(event.data.text).catch(err => {
                 console.error("Failed to copy text:", err);
                 // Fallback to backend invoke if needed, but usually navigator works
             });
        }
        return;
    }

    if (event.data.type === "toast") {
        if (event.data.toast) {
            const t = event.data.toast;
            const message = typeof t === "string" ? t : t.message;
            const timeout = typeof t === "object" ? t.timeout : 3000;
            showToast(message, { timeout, bottom: "40px" });
        }
        return;
    }

    if (event.data.type !== "preview-dblclick") return;
    if (event.data.id && currentItemId.value && event.data.id !== currentItemId.value) return;
    startEdit();
}

onMounted(() => {
    messageHandler = handlePostMessage;
    window.addEventListener("message", messageHandler);
});

onUnmounted(() => {
    resetLoader(); // Use resetLoader from composable instead
    
    if (messageHandler) {
        window.removeEventListener("message", messageHandler);
        messageHandler = null;
    }
    
    frameDblHandler = null;
});

defineExpose({
    resetState
});
</script>

<template>
    <div class="preview-pane">
        <div v-if="isLoading" class="loading-state">
            <div class="frame-shell">
                <pre v-if="loadingText" class="plain-text-preview">{{ loadingText }}</pre>
                <div v-else class="loading-placeholder">
                    <div class="spinner"></div>
                    <span>Loading preview...</span>
                </div>
            </div>
        </div>
        <div v-else-if="error && !isEditing" class="error-state">
            {{ error }}
        </div>
        <template v-else-if="isEditing">
            <div class="frame-shell">
                <div v-if="error" class="edit-error">{{ error }}</div>
                <textarea 
                    ref="editTextareaRef"
                    v-model="editedText" 
                    class="edit-textarea"
                    @keydown.escape.stop="cancelEdit"
                    :disabled="isSaving"
                ></textarea>
                <div class="edit-actions">
                    <button @click="cancelEdit" class="cancel-btn" :disabled="isSaving">Cancel</button>
                    <button @click="saveEdit" class="save-btn" :disabled="isSaving">
                        {{ isSaving ? 'Saving...' : 'Save' }}
                    </button>
                </div>
            </div>
        </template>
        <template v-else-if="previewContent">
            <div class="frame-shell" @dblclick.stop="startEdit">
                <template v-if="isAltPressed && plainTextContent">
                    <pre class="plain-text-preview">{{ plainTextContent }}</pre>
                </template>
                <template v-else>
                    <iframe 
                        class="content-frame"
                        ref="frameRef"
                        :srcdoc="previewContent"
                        sandbox="allow-same-origin allow-scripts"
                        @load="handleFrameLoad"
                    ></iframe>
                </template>
            </div>
        </template>
        <template v-else-if="isInline && !props.itemId">
            <div class="frame-shell placeholder-shell">
                <div class="placeholder-content">
                    <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256"><path fill="currentColor" d="M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM200,216H56V40h88V88a8,8,0,0,0,8,8h48V216Z"/></svg>
                    <span>Select an item to preview</span>
                </div>
            </div>
        </template>
        <PreviewFooter 
            v-if="!isEditing"
            :keyboard-state="keyboardState" 
            :is-editable="isEditable && !!previewContent"
            :is-inline="isInline"
        />
    </div>
</template>

<style scoped>
.preview-pane {
    height: 100%;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
}

.frame-shell {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
}

.content-frame {
    flex: 1;
    width: 100%;
    border: none;
    border-radius: 4px;
    background: transparent;
}

.loading-state,
.error-state,
.empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #6b7280);
    gap: 10px;
    font-size: 12px;
}

.loading-state {
    align-items: stretch;
    justify-content: flex-start;
    overflow: hidden;
}

.loading-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
}

.loading-text-preview {
    flex: 1;
    margin: 0;
    padding: 8px;
    font-family: ui-monospace, monospace;
    font-size: 0.8em;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
    overflow-y: auto;
    background: var(--bg-input, #ffffff);
    color: var(--text-primary, #111827);
    border-radius: 4px;
}

.plain-text-preview {
    flex: 1;
    margin: 0;
    padding: 8px;
    font-family: ui-monospace, monospace;
    font-size: 0.8em;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
    overflow-y: auto;
    background: var(--bg-input, #ffffff);
    color: var(--text-primary, #111827);
    border-radius: 4px;
}

.error-state {
    color: var(--error-color, #ef4444);
}

.spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color, #e0e0e0);
    border-radius: 50%;
    border-top-color: var(--accent, #3b82f6);
    animation: spin 1s linear infinite;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}

.edit-textarea {
    flex: 1;
    resize: none;
    padding: 8px;
    font-size: 0.8em;
    line-height: 1.4;
    border: none;
    background: var(--bg-input, #ffffff);
    color: var(--text-primary, #111827);
    font-family: ui-monospace, monospace;
    border-radius: 4px;
}

.edit-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.edit-textarea:focus { outline: none; }

.edit-error {
    padding: 6px 8px;
    background: var(--error-bg, #fef2f2);
    color: var(--error-color, #ef4444);
    border-radius: 4px;
    font-size: 0.75em;
    text-align: center;
}

.edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    padding: 6px 0;
}

.cancel-btn,
.save-btn {
    padding: 4px 10px;
    border: none;
    border-radius: 4px;
    font-size: 0.75em;
    cursor: pointer;
}

.cancel-btn:disabled,
.save-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.cancel-btn {
    background: var(--bg-secondary, #e5e7eb);
    color: var(--text-primary, #111827);
}

.save-btn {
    background: var(--accent, #3b82f6);
    color: var(--accent-text, #ffffff);
}

.placeholder-shell {
    align-items: center;
    justify-content: center;
    background: var(--bg-secondary, #f3f4f6);
    border-radius: 4px;
}

.placeholder-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary, #6b7280);
    font-size: 12px;
    opacity: 0.6;
}

.placeholder-content svg {
    opacity: 0.5;
}
</style>
