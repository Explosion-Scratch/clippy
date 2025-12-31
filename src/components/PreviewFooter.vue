<script setup>
import { computed } from "vue";
import { formatShortcut, ITEM_SHORTCUTS } from "../utils/itemShortcuts";

const props = defineProps({
    keyboardState: {
        type: Object,
        default: () => ({ currentlyPressed: [], itemShortcuts: [] })
    },
    isEditable: {
        type: Boolean,
        default: true
    },
    isInline: {
        type: Boolean,
        default: false
    }
});

const displayShortcuts = computed(() => {
    return props.keyboardState.itemShortcuts || [];
});

const showDefaultHint = computed(() => {
    return displayShortcuts.value.length === 0;
});
</script>

<template>
    <div class="preview-footer" :class="{ 'inline-footer': isInline }">
        <template v-if="showDefaultHint">
            <div v-if="isEditable" class="shortcut-group hint">
                <span class="shortcut-label">Double-click to edit</span>
            </div>
            <div class="shortcut-group">
                <span class="shortcut-label">{{ ITEM_SHORTCUTS.paste.label }}</span>
                <span class="shortcut-key">{{ formatShortcut(ITEM_SHORTCUTS.paste) }}</span>
            </div>
        </template>
        <template v-else>
            <div 
                v-for="shortcut in displayShortcuts" 
                :key="shortcut.id" 
                class="shortcut-group"
            >
                <span class="shortcut-label">{{ shortcut.label }}</span>
                <span class="shortcut-key">{{ formatShortcut(shortcut) }}</span>
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
    border-radius: 6px;
    gap: 8px;
}

.preview-footer.inline-footer {
    --inline-bg: #fff5;
    background-color: var(--inline-bg);
    border-top: none;
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
    
    .preview-footer.inline-footer {
        --inline-bg: #fff1;
        background-color: var(--inline-bg);
        border-top: none;
    }
}
</style>
