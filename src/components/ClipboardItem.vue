<script setup>
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
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

const emit = defineEmits(["delete", "mouseenter"]);

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
            label: `${item.formats.files.length} files`,
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

// Copy item to clipboard and paste
async function copyToClipboard() {
    try {
        console.log("Injecting item from ID:", props.item.id);
        const result = await invoke("inject_item", { id: props.item.id });
        console.log("Item injection result:", result);
    } catch (error) {
        console.error("Failed to inject item:", error);
    }
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
            return files[0].split("/").pop() || "File";
        }
        return `${files[0].split("/").pop()} +${files.length - 1} more`;
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
    return props.item.formats?.imageData || null;
});

const getIndexText = (idx) => {
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
        @click="copyToClipboard"
    >
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
            {{ getIndexText(item.index) || getInfoText(item) }}
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
}

.image-preview {
    width: 80px;
    height: 80px;
    object-fit: cover;
    flex-shrink: 0;
}

.content-preview {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    min-width: 0;
}

.preview-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.info {
    flex-shrink: 0;
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
}

.clipboard-item.is-selected .delete-btn {
    opacity: 1;
}


</style>
