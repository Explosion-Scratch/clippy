<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const previewContent = ref("");
const isLoading = ref(false);
const error = ref(null);
const currentId = ref(null);

async function openInDashboard() {
    if (currentId.value) {
        // Open dashboard with the item selected via backend command
        await invoke("open_in_dashboard", { id: currentId.value });
    }
}

async function fetchPreview(id) {
    currentId.value = id; // Store the current ID
    if (!id) {
        previewContent.value = "";
        return;
    }

    console.log("Fetching preview for id:", id);

    isLoading.value = true;
    error.value = null;
    previewContent.value = "";

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

                previewContent.value = html;
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
        <div v-else-if="previewContent" id="content" v-html="previewContent"></div>
        <div v-else class="empty-state">
            No item selected
        </div>
      <div class="footer">
        <div class="shortcut-group">
          <span>Inject</span>
          <span class="shortcut-key">⏎</span>
        </div>
        <div class="shortcut-group">
          <span>Copy</span>
          <span class="shortcut-key">⌘⏎</span>
        </div>
        <div class="action-button" @click="openInDashboard">
          <span>Open</span>
          <span class="shortcut-key">⇧⏎</span>
        </div>
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
        border-radius: 4px;
        overflow-y: scroll;
        margin-bottom: 10px;
        overflow-x: hidden;
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

:root {
    --footer-bg: #f3f4f6;
    --footer-border: #e5e7eb;
    --footer-text: #6b7280;
    --footer-text-hover: #111827;
}

@media (prefers-color-scheme: dark) {
    :root {
        --footer-bg: #00000033;
        --footer-border: #3e3e3e33;
        --footer-text: #9ca3af;
        --footer-text-hover: #f9fafb;
    }
}

.footer {
    height: 24px;
    background-color: var(--footer-bg);
    border-top: 1px solid var(--footer-border);
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    padding: 0 12px;
    font-size: 10px;
    color: var(--footer-text);
    user-select: none;
    flex-shrink: 0;
    font-family: system-ui, sans-serif;
    border-radius: 6px;

    .shortcut-group {
        display: flex;
        align-items: center;
        gap: 4px;
    }

    .shortcut-key {
        font-family: system-ui, sans-serif;
        opacity: 0.7;
    }

    .action-button {
        display: flex;
        align-items: center;
        gap: 4px;
        cursor: pointer;
        transition: color 0.15s;

        &:hover {
            color: var(--footer-text-hover);
        }
    }
}
</style>
