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
/* Light theme colors (default) */
:root {
    --bg-primary: transparent;
    --bg-secondary: transparent;
    --bg-tertiary: transparent;
    --bg-input: rgba(255, 255, 255, 0.3);
    --bg-status: rgba(255, 255, 255, 0.3);
    --text-primary: #333333;
    --text-secondary: #666666;
    --text-tertiary: #999999;
    --border-color: #e0e0e0;
    --border-light: #0003;
    --accent: lightseagreen;
    --accent-text: white;
    --accent-transparent: rgba(32, 178, 170, 0.2);
    --shadow-light: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    --shadow-medium: 0 2px 8px 0 rgb(0 0 0 / 0.1);
}

/* Solid colors for settings page */
.settings {
    --settings-bg-primary: #f8f9fa;
    --settings-bg-input: #ffffff;
    --settings-border-color: #dee2e6;
    --settings-shadow-light: 0 1px 3px 0 rgb(0 0 0 / 0.1);
    --settings-shadow-medium: 0 2px 8px 0 rgb(0 0 0 / 0.15);
}

/* Dark theme colors (applied via OS preference) */
@media (prefers-color-scheme: dark) {
    :root {
        --bg-primary: transparent;
        --bg-secondary: transparent;
        --bg-tertiary: transparent;
        --bg-input: rgba(0, 0, 0, 0.3);
        --bg-status: rgba(0, 0, 0, 0.3);
        --text-primary: #ffffff;
        --text-secondary: #cccccc;
        --text-tertiary: #999999;
        --border-color: #404040;
        --border-light: #ffffff3;
        --accent: #20b2aa;
        --accent-text: white;
        --accent-transparent: rgba(32, 178, 170, 0.3);
        --shadow-light: 0 1px 2px 0 rgb(0 0 0 / 0.3);
        --shadow-medium: 0 2px 8px 0 rgb(0 0 0 / 0.5);
    }

    /* Solid colors for settings page (dark theme) */
    .settings {
        --settings-bg-primary: #1e1e1e;
        --settings-bg-input: #2d2d2d;
        --settings-border-color: #404040;
        --settings-shadow-light: 0 1px 3px 0 rgb(0 0 0 / 0.3);
        --settings-shadow-medium: 0 2px 8px 0 rgb(0 0 0 / 0.5);
    }
}

input {
    background: var(--bg-input);
    border: 0.5px solid var(--border-light);
    border-radius: 5px;
    font-family: system-ui;
    box-shadow: var(--shadow-light);
    color: var(--text-primary);

    &::placeholder {
        color: var(--text-secondary);
        opacity: 0.7;
    }

    &:focus {
        outline: none;
        border-color: var(--accent);
        box-shadow: 0 0 0 3px var(--accent-transparent);
    }
}

html,
body {
    background: transparent !important;
    padding: 0 !important;
    margin: 0 !important;
    color: var(--text-primary);
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
