<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { listen, emit as tauriEmit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import PreviewPane from "./PreviewPane.vue";
import { showToast } from "../utils/ui";

const currentId = ref(null);
const keyboardState = ref({ currentlyPressed: [], itemShortcuts: [] });
let unlisten = null;
let keyboardStateUnlisten = null;

async function openInDashboard() {
    if (currentId.value) {
        await invoke("open_in_dashboard", { id: currentId.value });
    }
}

async function handleRefresh(newId) {
    if (newId && newId !== currentId.value) {
        currentId.value = newId;
        try {
            await tauriEmit("preview-item-changed", newId);
        } catch (e) {
            console.error("Failed to emit preview-item-changed:", e);
        }
    }
}

onMounted(async () => {
    unlisten = await listen("preview-item", (event) => {
        currentId.value = event.payload;
    });
    
    keyboardStateUnlisten = await listen("keyboard-state-changed", (event) => {
        keyboardState.value = event.payload || { currentlyPressed: [], itemShortcuts: [] };
    });
});

onUnmounted(() => {
    if (unlisten) {
        unlisten();
        unlisten = null;
    }
    if (keyboardStateUnlisten) {
        keyboardStateUnlisten();
        keyboardStateUnlisten = null;
    }
});
</script>

<template>
    <div id="wrapper" class="compact" v-if="currentId">
        <PreviewPane 
            :item-id="currentId" 
            :keyboard-state="keyboardState"
            :is-inline="false"
            @refresh="handleRefresh" 
        />
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
</style>

