<script setup>
import { ref, watch, nextTick, onUnmounted, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const EDIT_SAVE_TIMEOUT_MS = 10000;

const props = defineProps({
    itemId: {
        type: String,
        default: null
    }
});

const emit = defineEmits(["refresh"]);

const previewContent = ref("");
const isLoading = ref(false);
const error = ref(null);
const isEditing = ref(false);
const editedText = ref("");
const originalText = ref("");
const isEditable = ref(true);
const itemKind = ref("text");
const currentId = ref(null);
const frameRef = ref(null);
const isSaving = ref(false);

let abortController = null;
let frameDblHandler = null;
let messageHandler = null;

function resetState() {
    previewContent.value = "";
    error.value = null;
    isEditing.value = false;
    editedText.value = "";
    originalText.value = "";
    itemKind.value = "text";
    isEditable.value = true;
    isSaving.value = false;
}

function startEdit() {
    if (!originalText.value || !isEditable.value) return;
    isEditing.value = true;
}

function cancelEdit() {
    isEditing.value = false;
    editedText.value = originalText.value;
    error.value = null;
}

async function saveEdit() {
    if (!currentId.value || isSaving.value) return;
    
    isSaving.value = true;
    error.value = null;
    
    try {
        await invoke("edit_item", { 
            id: currentId.value, 
            formats: { text: editedText.value } 
        });
        isEditing.value = false;
        originalText.value = editedText.value;
        emit("refresh", currentId.value);
        await loadPreview(currentId.value);
    } catch (e) {
        console.error("Failed to save edit:", e);
        error.value = "Failed to save changes. Please try again.";
    } finally {
        isSaving.value = false;
    }
}

function abortPreviousRequest() {
    if (abortController) {
        abortController.abort();
        abortController = null;
    }
}

async function loadPreview(id) {
    abortPreviousRequest();
    
    currentId.value = id;

    if (!id) {
        resetState();
        return;
    }

    abortController = new AbortController();
    const signal = abortController.signal;

    isLoading.value = true;
    error.value = null;
    isEditing.value = false;
    previewContent.value = "";

    try {
        const data = await invoke("get_preview_content", { id });
        
        if (signal.aborted) return;

        itemKind.value = data.kind || "text";
        isEditable.value = itemKind.value !== "file" && itemKind.value !== "image";

        const formatsOrder = data.formatsOrder || [];
        const dataMap = data.data || {};

        let html = "";
        let text = "";
        for (const formatId of formatsOrder) {
            const formatData = dataMap[formatId];
            if (!formatData) continue;
            if (!html && formatData.html) html = formatData.html;
            if (!text && formatData.text) text = formatData.text;
        }

        if (html) {
            html = sanitizePreviewHtml(html, id);
        }

        if (signal.aborted) return;

        previewContent.value = html || "<div class='empty'>No preview available</div>";
        originalText.value = text || "";
        editedText.value = text || "";

        await nextTick();
        if (!signal.aborted) {
            attachFrameDblclick();
        }
    } catch (e) {
        if (signal.aborted) return;
        console.error("Failed to fetch preview:", e);
        error.value = "Failed to load preview";
    } finally {
        if (!signal.aborted) {
            isLoading.value = false;
        }
    }
}

function sanitizePreviewHtml(html, id) {
    if (html.includes('<iframe') && html.includes('src="')) {
        html = html.replace(/src="([^"]+)"/, (match, url) => {
            const separator = url.includes('?') ? '&' : '?';
            return `src="${url}${separator}interactive=false"`;
        });
    }
    
    html = html.replace('<html>', '<html class="compact">');
    
    const dblScript = `<` + 'script' + `>
window.addEventListener('dblclick', () => {
    try { parent.postMessage({ type: 'preview-dblclick', id: '${id}' }, '*'); } catch (_) {}
});
</` + 'script' + `>`;
    
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
    if (!doc) return;
    
    frameDblHandler = (event) => {
        event.preventDefault();
        startEdit();
    };
    doc.addEventListener("dblclick", frameDblHandler);
}

function handlePostMessage(event) {
    if (!event?.data || event.data.type !== "preview-dblclick") return;
    if (event.data.id && currentId.value && event.data.id !== currentId.value) return;
    startEdit();
}

onMounted(() => {
    messageHandler = handlePostMessage;
    window.addEventListener("message", messageHandler);
});

onUnmounted(() => {
    abortPreviousRequest();
    
    if (messageHandler) {
        window.removeEventListener("message", messageHandler);
        messageHandler = null;
    }
    
    frameDblHandler = null;
});
</script>

<template>
    <div class="preview-pane">
        <div v-if="isLoading" class="loading-state">
            <div class="spinner"></div>
            <span>Loading preview...</span>
        </div>
        <div v-else-if="error && !isEditing" class="error-state">
            {{ error }}
        </div>
        <template v-else-if="isEditing">
            <div v-if="error" class="edit-error">{{ error }}</div>
            <textarea 
                v-model="editedText" 
                class="edit-textarea"
                @keydown.escape="cancelEdit"
                :disabled="isSaving"
                autofocus
            ></textarea>
            <div class="edit-actions">
                <button @click="cancelEdit" class="cancel-btn" :disabled="isSaving">Cancel</button>
                <button @click="saveEdit" class="save-btn" :disabled="isSaving">
                    {{ isSaving ? 'Saving...' : 'Save' }}
                </button>
            </div>
        </template>
        <template v-else-if="previewContent">
            <div class="frame-shell" @dblclick.stop="startEdit">
                <iframe 
                    class="content-frame"
                    ref="frameRef"
                    :srcdoc="previewContent"
                    sandbox="allow-same-origin"
                ></iframe>
                <div v-if="isEditable" class="edit-hint">
                    <span>Double-click to edit</span>
                </div>
            </div>
        </template>
        <div v-else class="empty-state">
            No item selected
        </div>
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

.edit-hint {
    font-size: 0.7em;
    color: var(--text-secondary);
    text-align: center;
    padding: 2px 0;
    opacity: 0.6;
}
</style>
