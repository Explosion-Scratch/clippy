<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import PreviewPane from "./PreviewPane.vue";

const currentId = ref(null);
let unlisten = null;

async function openInDashboard() {
    if (currentId.value) {
        await invoke("open_in_dashboard", { id: currentId.value });
    }
}

onMounted(async () => {
    unlisten = await listen("preview-item", (event) => {
        currentId.value = event.payload;
    });
});

onUnmounted(() => {
    if (unlisten) {
        unlisten();
        unlisten = null;
    }
});
</script>

<template>
    <div id="wrapper" class="compact">
        <PreviewPane :item-id="currentId" />
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
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow-y: scroll;
    font-family: system-ui, sans-serif;
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
