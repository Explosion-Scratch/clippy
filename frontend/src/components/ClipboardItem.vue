<script setup>
import { computed } from 'vue'

const props = defineProps({
  item: {
    type: Object,
    required: true
  },
  selected: {
    type: Boolean,
    default: false
  }
})

defineEmits(['mouseenter'])

function formatTimestamp(timestamp) {
  const date = new Date(timestamp)
  const now = new Date()
  const diffMs = now - date
  const diffMins = Math.floor(diffMs / 60000)

  if (diffMins < 1) return 'now'
  if (diffMins < 60) return `${diffMins}m`
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours}h`
  const diffDays = Math.floor(diffHours / 24)
  if (diffDays < 7) return `${diffDays}d`
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}

function getPreviewText(item) {
  if (item.type === 'file' && item.files) {
    if (item.files.length === 1) {
      return `file://${item.files[0]}`
    }
    return `file://${item.files[0]} +${item.files.length - 1} more`
  }
  if (item.type === 'image') {
    return item.summary || 'Image'
  }
  if (!item.text) return 'No preview'
  return item.text.replace(/\n/g, ' ')
}

function getInfoText(item) {
  const copiesText = item.copies > 1 ? ` (${item.copies}×)` : ''
  return `${formatTimestamp(item.timestamp)}${copiesText}`
}

function getIndexText(idx, isSelected) {
  if (isSelected) return null
  if (idx === undefined || idx === null) return null
  if (idx > 9) return null
  return `⌘${idx === 9 ? 0 : idx + 1}`
}

const hasImage = computed(() => props.item.type === 'image')
</script>

<template>
  <div
    class="clipboard-item"
    :class="{ 'is-selected': selected, 'has-image': hasImage }"
    @mouseenter="$emit('mouseenter')"
  >
    <div v-if="hasImage" class="image-preview">
      <div class="image-placeholder">🖼️</div>
    </div>

    <div v-else class="content-preview">
      <div class="preview-text">{{ getPreviewText(item) }}</div>
    </div>

    <div class="info">
      {{ getIndexText(item.index, selected) || getInfoText(item) }}
    </div>
  </div>
</template>

<style scoped>
.clipboard-item {
  height: 23px;
  overflow: hidden;
  cursor: default;
  font-size: 0.8em;
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
  border-radius: 4px;
  padding: 1px 5px;
  color: var(--text-primary);
  transition: background var(--transition-fast);
}

.clipboard-item.has-image {
  height: 80px;
  padding-top: 4px;
  padding-bottom: 4px;
}

.clipboard-item:hover {
  background: var(--bg-secondary);
}

.clipboard-item.is-selected {
  background: var(--accent);
  color: var(--accent-text);
}

.clipboard-item.is-selected .info {
  color: var(--accent-text);
  opacity: 0.8;
}

.image-preview {
  width: 72px;
  height: 72px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  border-radius: 4px;
  flex-shrink: 0;
}

.image-placeholder {
  font-size: 2rem;
}

.content-preview {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
}

.preview-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.info {
  flex-shrink: 0;
  opacity: 0.6;
  color: var(--text-secondary);
  font-size: 0.9em;
}

.is-selected .info {
  color: var(--accent-text);
  opacity: 0.8;
}
</style>
