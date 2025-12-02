<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const previewHtml = ref("");
const isLoading = ref(false);
const error = ref(null);

async function fetchPreview(id) {
    if (!id) {
        previewHtml.value = "";
        return;
    }

    console.log("Fetching preview for id:", id);

    isLoading.value = true;
    error.value = null;
    previewHtml.value = "";

    try {
        // Fetch preview using Tauri command
        console.log("Invoking get_preview_content");
        const data = await invoke("get_preview_content", { id });
        console.log("Preview data received:", data);

        if (data.formatsOrder && data.formatsOrder.length > 0) {
            const preferredFormat = data.formatsOrder[0];
            const formatData = data.data[preferredFormat];
            if (formatData && formatData.html) {
                // Inject interactive=false into the iframe src if it's not already there
                // The backend might return a full HTML string or a URL. 
                // If it's a full HTML string containing an iframe, we might need to parse it.
                // However, the current implementation seems to return HTML content directly.
                // If the content is an iframe (like for images), we need to make sure the src has the param.

                let html = formatData.html;

                // Simple regex to append query param to src attribute if it's an iframe
                // This is a bit hacky but works for the current template system
                if (html.includes('<iframe') && html.includes('src="')) {
                    html = html.replace(/src="([^"]+)"/, (match, url) => {
                        const separator = url.includes('?') ? '&' : '?';
                        return `src="${url}${separator}interactive=false"`;
                    });
                }

                previewHtml.value = html;
                console.log("Preview HTML set");
            }
        } else {
            console.warn("No formats available for preview");
            error.value = "No preview available";
        }
    } catch (e) {
        console.error("Failed to fetch preview:", e);
        error.value = "Failed to load preview";
    } finally {
        isLoading.value = false;
    }
}

onMounted(async () => {
    console.log("PreviewWindow mounted");
    // Listen for preview requests from main window
    const unlisten = await listen("preview-item", (event) => {
        console.log("Received preview-item event:", event);
        const id = event.payload;
        fetchPreview(id);
    });
    onUnmounted(() => {
        unlisten();
    });
});
</script>

<template>
    <div id="wrapper" class="compact">
        <div v-if="isLoading" class="loading-state">
            <div class="spinner"></div>
            <span>Loading preview...</span>
        </div>
        <div v-else-if="error" class="error-state">
            {{ error }}
        </div>
        <div v-else-if="previewHtml" id="content" v-html="previewHtml"></div>
        <div v-else class="empty-state">
            No item selected
        </div>
    </div>
</template>

<style lang="less">
:root,
body,
#app {
    background: transparent !important;
    font-family: system-ui, sans-serif;
}

#wrapper {
    height: 100vh;
    width: 100vw;
    padding: 10px;
    /* Padding for the shadow/border */
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow-y: scroll;
    font-family: system-ui, sans-serif;
}

#content {
    flex: 1;
    width: 100%;
    height: 100%;

    iframe {
        width: 100%;
        height: 100%;
        border: none;
        background: white;
        /* Ensure iframe background is white */
    }
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

.spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color, #e0e0e0);
    border-radius: 50%;
    border-top-color: var(--accent, #3b82f6);
    animation: spin 1s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
