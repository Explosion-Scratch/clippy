<script setup>
import { ref, computed } from 'vue';

const props = defineProps({
  item: {
    type: Object,
    required: true
  }
});

const emit = defineEmits(['delete']);

const isExpanded = ref(false);
const isHovered = ref(false);

// Computed properties for formatting
const displayText = computed(() => {
  if (!props.item.text) return 'No text content';

  const text = props.item.text;
  if (text.length <= 150) return text;

  return isExpanded.value ? text : text.substring(0, 150) + '...';
});

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

const canExpand = computed(() => {
  return props.item.text && props.item.text.length > 150;
});

// Format timestamp
function formatTimestamp(timestamp) {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now - date;
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;

  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;

  return date.toLocaleDateString();
}

// Format byte size
function formatByteSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// Toggle expanded state
function toggleExpanded() {
  if (canExpand.value) {
    isExpanded.value = !isExpanded.value;
  }
}

// Handle delete
function handleDelete() {
  emit('delete', props.item.id);
}

// Copy text to clipboard
async function copyToClipboard() {
  try {
    if (props.item.text) {
      await navigator.clipboard.writeText(props.item.text);
      // Could add a toast notification here
    }
  } catch (error) {
    console.error('Failed to copy to clipboard:', error);
  }
}
</script>

<template>
  <div
    class="clipboard-item"
    :class="{ 'is-hovered': isHovered }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <div class="item-header">
      <div class="item-meta">
        <span class="timestamp">{{ formatTimestamp(item.timestamp) }}</span>
        <span class="size">{{ formatByteSize(item.byte_size) }}</span>
      </div>
      <div class="item-actions">
        <button
          v-if="hasText"
          @click="copyToClipboard"
          class="action-btn copy-btn"
          title="Copy to clipboard"
        >
          📋
        </button>
        <button
          @click="handleDelete"
          class="action-btn delete-btn"
          title="Delete item"
        >
          🗑️
        </button>
      </div>
    </div>

    <div class="item-content">
      <!-- Text content -->
      <div
        v-if="hasText"
        class="text-content"
        :class="{ 'is-clickable': canExpand }"
        @click="toggleExpanded"
      >
        <div class="text-wrapper">
          <span class="text">{{ displayText }}</span>
          <button
            v-if="canExpand"
            @click.stop="toggleExpanded"
            class="expand-btn"
          >
            {{ isExpanded ? 'Show less' : 'Show more' }}
          </button>
        </div>
      </div>

      <!-- Content type indicators -->
      <div class="content-types">
        <span v-if="hasHtml" class="content-type html-type">HTML</span>
        <span v-if="hasImage" class="content-type image-type">Image</span>
        <span v-if="hasFiles" class="content-type files-type">
          Files ({{ item.formats.files.length }})
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.clipboard-item {
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 16px;
  padding: 1rem;
  transition: all 0.2s ease;
  cursor: default;
}

.clipboard-item:hover {
  background: rgba(255, 255, 255, 0.95);
  border-color: rgba(0, 122, 255, 0.3);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  transform: translateY(-2px);
}

.item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.item-meta {
  display: flex;
  gap: 0.75rem;
  font-size: 0.875rem;
  color: #86868b;
}

.timestamp {
  font-weight: 500;
}

.size {
  opacity: 0.8;
}

.item-actions {
  display: flex;
  gap: 0.5rem;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.clipboard-item.is-hovered .item-actions {
  opacity: 1;
}

.action-btn {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.05);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: all 0.2s ease;
}

.action-btn:hover {
  background: rgba(0, 0, 0, 0.1);
  transform: scale(1.05);
}

.copy-btn:hover {
  background: rgba(0, 122, 255, 0.1);
}

.delete-btn:hover {
  background: rgba(255, 59, 48, 0.1);
}

.item-content {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.text-content {
  color: #1d1d1f;
  line-height: 1.5;
  word-break: break-word;
}

.text-content.is-clickable {
  cursor: pointer;
}

.text-wrapper {
  position: relative;
}

.text {
  font-size: 0.9375rem;
  white-space: pre-wrap;
}

.expand-btn {
  background: none;
  border: none;
  color: #007aff;
  cursor: pointer;
  font-size: 0.875rem;
  padding: 0.25rem 0;
  margin-top: 0.5rem;
  transition: color 0.2s ease;
}

.expand-btn:hover {
  color: #0056cc;
  text-decoration: underline;
}

.content-types {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.content-type {
  font-size: 0.75rem;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.html-type {
  background: rgba(255, 149, 0, 0.1);
  color: #ff9500;
}

.image-type {
  background: rgba(52, 199, 89, 0.1);
  color: #34c759;
}

.files-type {
  background: rgba(175, 82, 222, 0.1);
  color: #af52de;
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
  .clipboard-item {
    background: rgba(30, 30, 30, 0.8);
    border-color: rgba(255, 255, 255, 0.1);
    color: #f5f5f7;
  }

  .clipboard-item:hover {
    background: rgba(30, 30, 30, 0.95);
    border-color: rgba(0, 122, 255, 0.5);
  }

  .item-meta {
    color: #98989f;
  }

  .action-btn {
    background: rgba(255, 255, 255, 0.1);
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .text {
    color: #f5f5f7;
  }
}
</style>
