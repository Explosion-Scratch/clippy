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
:root {
    --accent: #0662;
    --accent-transparent: #066233;
}

input {
    --bg: #fff9;
    background: var(--bg);
    border: 0.5px solid #0003;
    border-radius: 5px;
    padding: 3px 6px;
    font-family: system-ui;
    box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    
    &:focus {
        box-shadow: 0 0 0 3px var(--accent-transparent);
        outline: none;
        border-color: var(--bg);
    }
}

input {
    --bg: #fff9;
    background: var(--bg);
    border: 0.5px solid var(--accent);
    border-radius: 5px;
    padding: 3px 6px;
    font-family: system-ui;
    box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    
    &:focus {
        box-shadow: 0 0 0 3px var(--accent);
        outline: none;
        border-color: var(--bg);
    }
}

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
