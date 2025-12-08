<script setup>
import { computed } from "vue";

const props = defineProps({
    item: {
        type: Object,
        required: true,
    },
    selected: {
        type: Boolean,
        default: false,
    },
});

const emit = defineEmits(["delete", "mouseenter", "select"]);

// Format timestamp for display
function formatTimestamp(timestamp) {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "now";
    if (diffMins < 60) return `${diffMins}m`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d`;
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

// Format first copied timestamp for display
function formatFirstCopied(firstCopied) {
    const date = new Date(firstCopied * 1000);
    return date.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
    });
}

// Format byte size to be more compact
function formatByteSize(bytes) {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}K`;
    return `${Math.round(bytes / (1024 * 1024))}M`;
}

// Get content type
function getContentType(item) {
    if (item.formats?.files && item.formats.files.length > 0) {
        return {
            type: "files",
            label: `${item.formats.files.length} file${item.formats.files.length > 1 ? "s" : ""}`,
        };
    }
    if (item.formats?.imageData) {
        return { type: "image", label: "Image" };
    }
    if (item.formats?.html) {
        return { type: "html", label: "HTML" };
    }
    return { type: "text", label: "Text" };
}

function getItemIcon(item) {
    const text = item.text?.trim() || "";
    
    if (item.formats?.files && item.formats.files.length > 0) {
        return { type: "svg", name: "folder" };
    }
    if (item.formats?.imageData) {
        return { type: "svg", name: "image" };
    }
    
    const hexColorRe = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;
    const rgbRe = /^rgba?\s*\(/i;
    const hslRe = /^hsla?\s*\(/i;
    if (hexColorRe.test(text) || rgbRe.test(text) || hslRe.test(text)) {
        return { type: "color", color: text };
    }
    
    if (text.startsWith("http://") || text.startsWith("https://")) {
        return { type: "svg", name: "link" };
    }
    
    if (item.formats?.html) {
        return { type: "svg", name: "code" };
    }
    
    return { type: "svg", name: "file-text" };
}

const itemIcon = computed(() => getItemIcon(props.item));

// Select item (notify parent to paste)
function selectItem() {
    emit("select");
}

// Handle delete
function handleDelete() {
    emit("delete", props.item.id);
}

// Get preview text
function getPreviewText(item) {
    if (item.formats?.files && item.formats.files.length > 0) {
        const files = item.formats.files;
        if (files.length === 1) {
            const path = files[0];
            return `file://${path}`;
        }
        const firstPath = files[0];
        return `file://${firstPath} +${files.length - 1} more`;
    }

    if (!item.text) return "No preview";

    return item.text;
}

// Get info text for the item
function getInfoText(item) {
    const copiesText = item.copies > 1 ? ` (${item.copies}×)` : "";
    const timestampText = formatTimestamp(item.timestamp);
    return `${timestampText}${copiesText}`;
}

// Computed properties
const hasImage = computed(() => {
    return props.item.formats?.imageData;
});

const imageDataUrl = computed(() => {
    return props.item.data || null;
});

const getIndexText = (idx, isSelected) => {
    if (isSelected) {
        return null;
    }
    if (!idx && idx !== 0) {
        return null;
    }
    if (idx > 9) {
        return null;
    }
    return `⌘${idx === 9 ? 0 : idx + 1}`;
};
</script>

<template>
    <div
        class="clipboard-item"
        :class="{ 'is-selected': props.selected }"
        @mouseenter="$emit('mouseenter')"
        @click="selectItem"
    >
        <div class="item-icon" v-if="!hasImage">
            <span v-if="itemIcon.type === 'color'" class="color-swatch" :style="{ backgroundColor: itemIcon.color }"></span>
            <svg v-else-if="itemIcon.name === 'folder'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 256 256"><path fill="currentColor" d="M216,72H130.67L102.93,51.2a16.12,16.12,0,0,0-9.6-3.2H40A16,16,0,0,0,24,64V200a16,16,0,0,0,16,16H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg>
            <svg v-else-if="itemIcon.name === 'image'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 256 256"><path fill="currentColor" d="M216,40H40A16,16,0,0,0,24,56V200a16,16,0,0,0,16,16H216a16,16,0,0,0,16-16V56A16,16,0,0,0,216,40ZM148,108a8,8,0,1,1,8-8A8,8,0,0,1,148,108Zm52,92H56a8,8,0,0,1-5.66-13.66l33-33a8,8,0,0,1,11.32,0L116,174.63l49.17-49.17a8,8,0,0,1,11.32,0L213.66,162.63A8,8,0,0,1,208,176Z"/></svg>
            <svg v-else-if="itemIcon.name === 'link'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 256 256"><path fill="currentColor" d="M137.54,186.36a8,8,0,0,1,0,11.31l-9.94,10a56,56,0,0,1-79.22-79.27l24.12-24.12a56,56,0,0,1,76.81-2.28,8,8,0,1,1-10.64,12,40,40,0,0,0-54.85,1.63L59.7,139.72a40,40,0,0,0,56.58,56.58l9.94-9.94A8,8,0,0,1,137.54,186.36Zm70.08-138a56.08,56.08,0,0,0-79.22,0l-9.94,9.95a8,8,0,0,0,11.32,11.31l9.94-9.94a40,40,0,0,1,56.58,56.58l-24.11,24.12a40,40,0,0,1-54.85,1.63,8,8,0,1,0-10.64,12,56,56,0,0,0,76.81-2.28l24.12-24.12A56.08,56.08,0,0,0,207.62,48.38Z"/></svg>
            <svg v-else-if="itemIcon.name === 'code'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 256 256"><path fill="currentColor" d="M69.12,94.15,28.5,128l40.62,33.85a8,8,0,1,1-10.24,12.29l-48-40a8,8,0,0,1,0-12.29l48-40a8,8,0,0,1,10.24,12.3Zm176,27.7-48-40a8,8,0,1,0-10.24,12.3L227.5,128l-40.62,33.85a8,8,0,1,0,10.24,12.29l48-40a8,8,0,0,0,0-12.29ZM162.73,32.48a8,8,0,0,0-10.25,4.79l-64,176a8,8,0,0,0,4.79,10.26A8.14,8.14,0,0,0,96,224a8,8,0,0,0,7.52-5.27l64-176A8,8,0,0,0,162.73,32.48Z"/></svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 256 256"><path fill="currentColor" d="M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM200,216H56V40h88V88a8,8,0,0,0,8,8h48V216Z"/></svg>
        </div>

        <!-- Image preview for images -->
        <img
            v-if="hasImage && imageDataUrl"
            :src="imageDataUrl"
            class="image-preview"
            alt="Image preview"
        />

        <!-- Content preview for non-images -->
        <div v-else class="content-preview">
            <div class="preview-text">{{ getPreviewText(item) }}</div>
        </div>

        <!-- Size info -->
        <div class="info">
            {{ getIndexText(item.index, props.selected) || getInfoText(item) }}
        </div>
    </div>
</template>

<style scoped>
.clipboard-item {
    height: 80px;
    cursor: pointer;
    user-select: none;
    position: relative;
    display: flex;
    align-items: center;
    overflow: hidden;
    color: var(--text-primary);
    background: var(--bg-primary);
    border-radius: 4px;
}

.clipboard-item:hover {
    background: var(--bg-secondary);
}

.clipboard-item.is-selected {
    background: var(--accent);
    color: var(--accent-text);
}

.image-preview {
    width: 80px;
    height: 80px;
    object-fit: cover;
    flex-shrink: 0;
    border-radius: 2px;
}

.content-preview {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0 8px;
}

.preview-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: inherit;
}

.item-icon {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: 2px;
    opacity: 0.6;
}

.item-icon svg {
    width: 12px;
    height: 12px;
}

.color-swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    border: 1px solid rgba(0, 0, 0, 0.2);
}

.info {
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 0.8em;
}

.delete-btn {
    width: 20px;
    height: 20px;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    opacity: 0;
    color: var(--text-secondary);
}

.delete-btn:hover {
    color: var(--text-primary);
}

.clipboard-item.is-selected .delete-btn {
    opacity: 1;
    color: var(--accent-text);
}

.clipboard-item.is-selected .delete-btn:hover {
    color: var(--accent-text);
}
</style>
