<script setup>
import { ref, computed } from "vue";
const props = defineProps({
    item: {
        type: Object,
        required: true,
    },
});

const emit = defineEmits(["delete"]);
const isHovered = ref(false);

// Format timestamp for display
function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
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
            icon: "📁",
            label: `${item.formats.files.length} files`,
        };
    }
    if (item.formats?.imageData) {
        return { type: "image", icon: "🖼️", label: "Image" };
    }
    if (item.formats?.html) {
        return { type: "html", icon: "🌐", label: "HTML" };
    }
    return { type: "text", icon: "📝", label: "Text" };
}

// Copy text to clipboard
async function copyToClipboard() {
    try {
        if (props.item.text) {
            await navigator.clipboard.writeText(props.item.text);
        }
    } catch (error) {
        console.error("Failed to copy to clipboard:", error);
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

    const text = item.text;
    const maxLength = 60;
    if (text.length <= maxLength) return text;

    return text.substring(0, maxLength).trim() + "...";
}

// Computed properties
const hasText = computed(() => {
    return props.item.formats?.txt || props.item.text;
});

const hasHtml = computed(() => {
    return props.item.formats?.html;
});

const hasImage = computed(() => {
    return props.item.formats?.imageData;
});

const hasFiles = computed(() => {
    return props.item.formats?.files && props.item.formats.files.length > 0;
});

const displayText = computed(() => {
    if (!props.item.text) return "No text content";
    const text = props.item.text;
    if (text.length <= 150) return text;
    return text.substring(0, 150) + "...";
});
</script>

<template>
    <div
        class="clipboard-item"
        :class="{ 'is-hovered': isHovered }"
        @mouseenter="isHovered = true"
        @mouseleave="isHovered = false"
        @click="copyToClipboard"
    >
        <!-- Main content area -->
        <div class="item-main">
            <div class="item-info">
                <span class="content-icon">{{
                    getContentType(item).icon
                }}</span>
                <div class="text-preview">{{ getPreviewText(item) }}</div>
            </div>
        </div>

        <!-- Footer with metadata and actions -->
        <div class="item-footer">
            <div class="metadata">
                <span class="timestamp">{{
                    formatTimestamp(item.timestamp)
                }}</span>
                <span class="separator">•</span>
                <span class="size">{{ formatByteSize(item.byte_size) }}</span>
            </div>

            <div class="actions" v-show="isHovered">
                <button
                    @click.stop="handleDelete"
                    class="delete-btn"
                    title="Delete"
                >
                    ✕
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.clipboard-item {
    background: rgba(255, 255, 255, 0.8);
    border: 1px solid rgba(0, 0, 0, 0.08);
    border-radius: 8px;
    padding: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
    position: relative;
}

.clipboard-item:hover {
    background: rgba(255, 255, 255, 0.95);
    border-color: rgba(0, 122, 255, 0.3);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.clipboard-item:active {
    transform: scale(0.98);
}

.item-main {
    margin-bottom: 6px;
    min-height: 16px;
}

.item-info {
    display: flex;
    align-items: center;
    gap: 8px;
}

.content-icon {
    font-size: 14px;
    opacity: 0.7;
    flex-shrink: 0;
}

.text-preview {
    font-size: 13px;
    font-weight: 400;
    color: #1d1d1f;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    line-height: 1.2;
}

.item-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.metadata {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: #8e8e93;
}

.timestamp {
    font-variant-numeric: tabular-nums;
    font-weight: 500;
}

.size {
    font-variant-numeric: tabular-nums;
    font-weight: 400;
}

.separator {
    opacity: 0.5;
}

.actions {
    display: flex;
    gap: 4px;
}

.delete-btn {
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 4px;
    background: rgba(255, 59, 48, 0.1);
    color: #ff3b30;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    line-height: 1;
}

.delete-btn:hover {
    background: rgba(255, 59, 48, 0.2);
    transform: scale(1.1);
}

.delete-btn:active {
    transform: scale(0.95);
}

/* Content type specific styles */
.clipboard-item[data-content-type="files"] .text-preview {
    color: #af52de;
}

.clipboard-item[data-content-type="image"] .text-preview {
    color: #34c759;
}

.clipboard-item[data-content-type="html"] .text-preview {
    color: #ff9500;
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
    .clipboard-item {
        background: rgba(44, 44, 46, 0.8);
        border-color: rgba(255, 255, 255, 0.1);
    }

    .clipboard-item:hover {
        background: rgba(58, 58, 60, 0.9);
        border-color: rgba(0, 122, 255, 0.4);
    }

    .text-preview {
        color: #f5f5f7;
    }

    .metadata {
        color: #98989f;
    }

    .separator {
        opacity: 0.3;
    }
}
</style>
