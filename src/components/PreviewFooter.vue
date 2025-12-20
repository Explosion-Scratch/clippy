<script setup>
import { computed } from "vue";
import { formatShortcut } from "../utils/itemShortcuts";

const props = defineProps({
    keyboardState: {
        type: Object,
        default: () => ({ currentlyPressed: [], itemShortcuts: [] })
    },
    isEditable: {
        type: Boolean,
        default: true
    }
});

const displayShortcuts = computed(() => {
    return props.keyboardState.itemShortcuts || [];
});
</script>

<template>
    <div class="preview-footer">
        <template v-if="displayShortcuts.length > 0">
            <div 
                v-for="shortcut in displayShortcuts" 
                :key="shortcut.id" 
                class="shortcut-group"
            >
                <span class="shortcut-label">{{ shortcut.label }}</span>
                <span class="shortcut-key">{{ formatShortcut(shortcut) }}</span>
            </div>
        </template>
        <template v-else>
            <div class="shortcut-group hint">
                <span v-if="isEditable" class="shortcut-label">Double-click to edit</span>
                <span v-else class="shortcut-label">Preview</span>
            </div>
        </template>
    </div>
</template>

<style scoped>
.preview-footer {
    height: 24px;
    background-color: var(--footer-bg, #f3f4f6);
    border-top: 1px solid var(--footer-border, #e5e7eb);
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    padding: 0 12px;
    font-size: 10px;
    color: var(--footer-text, #6b7280);
    user-select: none;
    flex-shrink: 0;
    font-family: system-ui, sans-serif;
    border-radius: 0 0 6px 6px;
    gap: 8px;
}

.shortcut-group {
    display: flex;
    align-items: center;
    gap: 4px;
}

.shortcut-group.hint {
    opacity: 0.7;
}

.shortcut-label {
    white-space: nowrap;
}

.shortcut-key {
    font-family: system-ui, sans-serif;
    opacity: 0.7;
}

@media (prefers-color-scheme: dark) {
    .preview-footer {
        --footer-bg: #00000033;
        --footer-border: #3e3e3e33;
        --footer-text: #9ca3af;
    }
}
</style>
