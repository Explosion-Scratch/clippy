<template>
  <div class="flex-1 flex flex-col border-r border-gray-200 min-w-[350px] max-w-[450px] bg-white">
    <div 
      v-if="selectedIds.size > 0" 
      class="bg-blue-50/50 px-4 py-2 text-sm text-blue-700 flex items-center justify-between border-b border-blue-100 backdrop-blur-sm"
    >
      <span class="font-medium">{{ selectedIds.size }} selected</span>
      <div class="flex gap-3">
        <button @click="$emit('clear-selection')" class="hover:text-blue-800 text-xs uppercase tracking-wide font-semibold">
          Clear
        </button>
        <button @click="$emit('delete-selected')" class="hover:text-red-600 text-red-500 text-xs uppercase tracking-wide font-semibold">
          Delete
        </button>
      </div>
    </div>

    <div 
      class="flex-1 overflow-y-auto scrollbar-thin" 
      @scroll="$emit('scroll', $event)"
      @mousedown="handleMouseDown"
      @mousemove="handleMouseMove"
      @mouseup="handleMouseUp"
      @mouseleave="handleMouseUp"
    >
      <div v-if="items.length === 0 && !loading" class="flex flex-col items-center justify-center h-64 text-gray-400">
        <PhGhost :size="48" class="mb-3 opacity-50" />
        <p class="text-sm">No items found</p>
      </div>

      <div 
        v-for="(item, index) in items" 
        :key="item.id"
        :id="'item-' + item.id"
        :data-item-id="item.id"
        @click="$emit('select', item, $event)"
        class="group flex items-center gap-3 px-4 py-2 cursor-pointer border-b border-gray-50 hover:bg-gray-50 transition-all relative select-none"
        :class="{
          'bg-stone-100': selectedItem?.id === item.id,
          'bg-stone-100/50': selectedIds.has(item.id) && selectedItem?.id !== item.id
        }"
      >
        <div 
          class="absolute left-0 top-0 bottom-0 w-1 bg-blue-500 transform scale-y-0 transition-transform"
          :class="{'scale-y-100': selectedItem?.id === item.id}"
        />

        <div 
          class="text-[10px] font-mono text-gray-400 w-8 text-right hover:text-blue-500 hover:underline cursor-copy"
          @click.stop="copyIndexWithToast(item)"
          title="Click to copy index"
        >
          #{{ item._index !== undefined ? item._index : item.index }}
        </div>

        <div class="w-5 h-5 rounded flex items-center justify-center bg-gray-100 text-gray-500 text-xs flex-shrink-0">
          <PhTextT v-if="item.type === 'text'" :size="14" />
          <PhImageIcon v-else-if="item.type === 'image'" :size="14" />
          <PhFileIcon v-else-if="item.type === 'file'" :size="14" />
          <PhCube v-else :size="14" />
        </div>

        <div class="flex-1 min-w-0">
          <div class="text-sm text-gray-900 truncate font-normal">
            {{ item.summary || item.data }}
          </div>
        </div>
        
        <div class="text-[10px] text-gray-400 tabular-nums whitespace-nowrap">
          {{ formatBytes(item.size) }}
        </div>
      </div>

      <div v-if="loadingMore" class="py-4 flex justify-center">
        <div class="spinner w-4 h-4" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { watch, nextTick, ref } from 'vue'
import { PhTextT, PhImage as PhImageIcon, PhFile as PhFileIcon, PhCube, PhGhost } from '@phosphor-icons/vue'

const props = defineProps({
  items: Array,
  selectedItem: Object,
  selectedIds: Set,
  loadingMore: Boolean,
  loading: Boolean
})

const emit = defineEmits(['select', 'copy-index', 'toggle-select', 'scroll', 'delete-selected', 'clear-selection', 'toast', 'multi-select'])

const isDragging = ref(false)
const dragStartId = ref(null)
const dragStartPos = ref(null)
const DRAG_THRESHOLD = 10

watch(() => props.selectedItem, (newItem) => {
  if (newItem) {
    nextTick(() => {
      const el = document.getElementById('item-' + newItem.id)
      if (el) {
        el.scrollIntoView({ block: 'nearest' })
      }
    })
  }
})

const handleMouseDown = (e) => {
  const itemEl = e.target.closest('[data-item-id]')
  if (itemEl && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    dragStartId.value = itemEl.dataset.itemId
    dragStartPos.value = { x: e.clientX, y: e.clientY }
  }
}

const handleMouseMove = (e) => {
  if (!dragStartId.value) return
  
  if (!isDragging.value && dragStartPos.value) {
    const dx = Math.abs(e.clientX - dragStartPos.value.x)
    const dy = Math.abs(e.clientY - dragStartPos.value.y)
    const distance = Math.sqrt(dx * dx + dy * dy)
    
    if (distance >= DRAG_THRESHOLD) {
      isDragging.value = true
    } else {
      return
    }
  }
  
  const itemEl = e.target.closest('[data-item-id]')
  if (itemEl) {
    const currentId = itemEl.dataset.itemId
    if (currentId && dragStartId.value) {
      const startIdx = props.items.findIndex(i => i.id === dragStartId.value)
      const currentIdx = props.items.findIndex(i => i.id === currentId)
      
      if (startIdx !== -1 && currentIdx !== -1) {
        const start = Math.min(startIdx, currentIdx)
        const end = Math.max(startIdx, currentIdx)
        
        const idsToSelect = props.items.slice(start, end + 1).map(i => i.id)
        emit('multi-select', idsToSelect)
      }
    }
  }
}

const handleMouseUp = () => {
  isDragging.value = false
  dragStartId.value = null
  dragStartPos.value = null
}

const copyIndexWithToast = (item) => {
  const index = item._index !== undefined ? item._index : item.index
  navigator.clipboard.writeText(String(index)).then(() => {
    emit('toast', { title: 'Copied', message: `Index ${index} copied`, type: 'success' })
  })
}

const formatBytes = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}
</script>

<style scoped>
.spinner {
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-left-color: #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
