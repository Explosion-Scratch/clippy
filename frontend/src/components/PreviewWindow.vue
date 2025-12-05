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

    <div class="preview-content compact">
      <div v-if="!item" class="empty-state">
        <span class="empty-icon">👆</span>
        <p>Select an item to preview</p>
      </div>

      <div v-else-if="currentPreviewData" class="preview-html" v-html="currentPreviewData.html.replaceAll(':root {', '.preview-html {').replaceAll('body {', '.preview-html {').replaceAll('html {', '.preview-html {')"></div>

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
  height: 100%;
  width: 100%;
  padding: 10px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  font-family: system-ui, sans-serif;
  background: transparent;
  border-radius: 12px;
}

.format-tabs {
  display: none; /* Real app doesn't show tabs in the preview window code I read */
}

.preview-content {
  flex: 1;
  overflow-y: auto;
  padding-bottom: 10px;
  position: relative;
}

.preview-content iframe {
  border: none;
  border-radius: 4px;
  overflow-y: auto;
  margin-bottom: 10px;
  overflow-x: hidden;
  width: 100%;
  height: 100%;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary, #6b7280);
  gap: 10px;
  font-size: 12px;
}

.empty-icon {
  font-size: 2rem;
  display: none; /* Real app doesn't have icon in empty state text */
}

.preview-html {
  font-size: 0.875rem;
  line-height: 1.6;
  color: var(--text-primary);
}

.footer {
  height: 24px;
  background-color: var(--bg-tertiary, #f3f4f6);
  border-top: 1px solid var(--border-subtle, #e5e7eb);
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  padding: 0 12px;
  font-size: 10px;
  color: var(--text-secondary, #6b7280);
  user-select: none;
  flex-shrink: 0;
  font-family: system-ui, sans-serif;
  border-radius: 6px;
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
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  transition: color 0.15s;
}

.action-button:hover {
  color: var(--text-primary);
}
</style>
