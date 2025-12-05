<script setup>
import { computed, ref } from 'vue'

const props = defineProps({
  item: {
    type: Object,
    default: null
  },
  preview: {
    type: Object,
    default: null
  }
})

const activeFormat = ref(null)

const currentFormat = computed(() => {
  if (!props.preview) return null
  if (activeFormat.value && props.preview.data[activeFormat.value]) {
    return activeFormat.value
  }
  return props.preview.formatsOrder?.[0] || null
})

const currentPreviewData = computed(() => {
  if (!props.preview || !currentFormat.value) return null
  return props.preview.data[currentFormat.value]
})

const availableFormats = computed(() => {
  return props.preview?.formatsOrder || []
})

function setFormat(format) {
  activeFormat.value = format
}
</script>

<template>
  <div class="preview-window">
    <div v-if="availableFormats.length > 1" class="format-tabs">
      <button 
        v-for="fmt in availableFormats" 
        :key="fmt"
        class="format-tab"
        :class="{ active: fmt === currentFormat }"
        @click="setFormat(fmt)"
      >
        {{ fmt }}
      </button>
    </div>

    <div class="preview-content">
      <div v-if="!item" class="empty-state">
        <span class="empty-icon">👆</span>
        <p>Select an item to preview</p>
      </div>

      <div v-else-if="currentPreviewData" class="preview-html" v-html="currentPreviewData.html"></div>

      <div v-else class="empty-state">
        <span class="empty-icon">📋</span>
        <p>{{ item.summary || 'No preview available' }}</p>
      </div>
    </div>

    <div class="footer">
      <div class="shortcut-group">
        <span>Inject</span>
        <span class="shortcut-key">⏎</span>
      </div>
      <div class="shortcut-group">
        <span>Copy</span>
        <span class="shortcut-key">⌘⏎</span>
      </div>
      <div class="shortcut-group action-button">
        <span>Open</span>
        <span class="shortcut-key">⇧⏎</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.preview-window {
  width: 400px;
  min-height: 400px;
  background: var(--bg-primary);
  border-radius: 0 12px 12px 0;
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  font-family: system-ui, sans-serif;
  border-left: 1px solid var(--border-subtle);
}

.format-tabs {
  display: flex;
  gap: 2px;
  padding: 8px 12px 0;
  border-bottom: 1px solid var(--border-subtle);
}

.format-tab {
  padding: 6px 12px;
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 0.75rem;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all var(--transition-fast);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.format-tab:hover {
  color: var(--text-primary);
}

.format-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.preview-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  position: relative;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 200px;
  color: var(--text-muted);
  text-align: center;
  gap: 10px;
  font-size: 12px;
}

.empty-icon {
  font-size: 2rem;
}

.preview-html {
  font-size: 0.875rem;
  line-height: 1.6;
  color: var(--text-primary);
}

.preview-html :deep(pre) {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 0;
}

.preview-html :deep(code) {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.preview-html :deep(a) {
  color: var(--accent);
  text-decoration: none;
}

.preview-html :deep(a:hover) {
  text-decoration: underline;
}

.preview-html :deep(.file-list) {
  list-style: none;
  padding: 0;
  margin: 0;
}

.preview-html :deep(.file-list li) {
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 4px;
  margin-bottom: 4px;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.preview-html :deep(.image-preview) {
  text-align: center;
  padding: 40px 20px;
  background: var(--bg-secondary);
  border-radius: 8px;
}

.preview-html :deep(.placeholder) {
  font-size: 1.5rem;
}

.footer {
  height: 24px;
  background: var(--bg-tertiary);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  padding: 0 12px;
  font-size: 10px;
  color: var(--text-muted);
  user-select: none;
  flex-shrink: 0;
  font-family: system-ui, sans-serif;
  border-radius: 0 0 12px 0;
}

.shortcut-group {
  display: flex;
  align-items: center;
  gap: 4px;
}

.shortcut-key {
  font-family: system-ui, sans-serif;
  opacity: 0.7;
}

.action-button {
  cursor: pointer;
  transition: color var(--transition-fast);
}

.action-button:hover {
  color: var(--text-primary);
}

@media (max-width: 900px) {
  .preview-window {
    width: 100%;
    max-width: 400px;
    border-radius: 0 0 12px 12px;
    border-left: none;
    border-top: 1px solid var(--border-subtle);
  }
  
  .footer {
    border-radius: 0 0 12px 12px;
  }
}
</style>
