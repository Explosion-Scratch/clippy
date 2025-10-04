<script setup>
import { onMounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

onMounted(() => {
    // Close window on Escape key for main window
    document.addEventListener("keyup", async (e) => {
        if (e.key === "Escape") {
            await invoke("hide_app");
        }
    });
});
</script>

<template>
    <div id="app">
        <router-view />
    </div>
</template>

<style lang="less">
html,
body {
    background: transparent !important;
    padding: 0 !important;
    margin: 0 !important;
}

* {
    box-sizing: border-box;
}

html,
body,
#app {
    overflow: hidden;
    height: 100vh;
    width: 100vw;
}
</style>
