<template>
  <div class="flex-[2] bg-white flex flex-col h-full overflow-hidden relative">
    <div v-if="!item" class="absolute inset-0 flex flex-col items-center justify-center text-gray-300 select-none">
      <PhMouseSimple :size="64" class="mb-4 opacity-20" />
      <p class="text-sm text-gray-400">Select an item to view details</p>
    </div>

    <div v-else class="flex flex-col h-full">
      <div class="px-6 py-4 border-b border-gray-200 bg-white flex-shrink-0">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-gradient-to-br from-gray-100 to-gray-50 rounded-lg shadow-sm">
              <PhTextT v-if="item.type === 'text'" :size="24" />
              <PhImageIcon v-else-if="item.type === 'image'" :size="24" />
              <PhFileIcon v-else-if="item.type === 'file'" :size="24" />
              <PhCube v-else :size="24" />
            </div>
            <div>
              <h1 class="text-lg font-semibold text-gray-900 capitalize">
                {{ item.type }} Item
              </h1>
              <div class="text-xs text-gray-500 flex items-center gap-2">
                <span>{{ new Date(item.date).toLocaleString() }}</span>
                <span>•</span>
                <span>{{ formatBytes(item.size) }}</span>
                <span v-if="item.copyCount">•</span>
                <span v-if="item.copyCount">{{ item.copyCount }} {{ item.copyCount === 1 ? 'copy' : 'copies' }}</span>
                <span>•</span>
                <span 
                  class="font-mono cursor-pointer hover:text-blue-500 hover:underline" 
                  @click.stop="copyTextWithToast(item.id)"
                  title="Click to copy hash"
                >
                  {{ item.id.substring(0, 8) }}
                </span>
              </div>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <button 
              @click="$emit('copy', item.id)" 
              class="btn-icon bg-gradient-to-r from-blue-600 to-blue-500 hover:from-blue-700 hover:to-blue-600 text-white"
              title="Copy Item"
            >
              <PhCopy :size="18" />
            </button>
            <button 
              @click="$emit('delete', item.id)" 
              class="btn-icon bg-white border border-gray-200 hover:bg-red-50 hover:border-red-200 text-red-600"
              title="Delete Item"
            >
              <PhTrash :size="18" />
            </button>
          </div>
        </div>

        <div v-if="fullData?.formats" class="flex items-end gap-1 overflow-x-auto scrollbar-thin">
          <button 
            v-for="(fmt, idx) in fullData.formats" 
            :key="idx"
            @click="$emit('format-change', idx)"
            class="px-3 py-1.5 text-xs font-medium rounded-t-lg border-t border-x transition-all flex items-center gap-2"
            :class="activeFormatIndex === idx 
              ? 'bg-white border-gray-200 text-gray-900 relative top-[1px] shadow-sm' 
              : 'border-transparent text-gray-500 hover:bg-gray-50'"
          >
            <span>{{ fmt.pluginId }}</span>
            <span class="text-[10px] bg-gray-200 px-1.5 py-0.5 rounded text-gray-600">
              {{ getFormatSize(fmt) }}
            </span>
          </button>
        </div>
      </div>

      <div class="flex-1 bg-gray-50 p-6 min-h-0 flex flex-col">
        <div v-if="loading" class="flex items-center justify-center h-full">
          <div class="spinner w-8 h-8" />
        </div>
        
        <div v-else-if="fullData?.formats?.[activeFormatIndex]" class="flex-1 bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden flex flex-col max-w-4xl mx-auto w-full">
          <div class="flex-1 overflow-y-auto p-1 relative scrollbar-thin">
            <!-- Image -->
            <div 
              v-if="currentFormat.pluginId === 'image'" 
              class="flex flex-col items-center justify-center min-h-full bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PHBhdGggZD0iTTEwIDBoMTB2MTBIMTB6TTAgMTBoMTB2MTBIMHoiIGZpbGw9IiNmYWZhZmEiIGZpbGwtb3BhY2l0eT0iMSIvPjwvc3ZnPg==')]"
            >
              <img 
                :src="imageBlobUrl || getImageSrc(currentFormat)" 
                class="max-w-full max-h-full object-contain shadow-lg rounded cursor-pointer"
                alt="Clipboard image"
                @click="openImageInNewTab"
              />
            </div>
            
            <!-- HTML -->
            <div v-else-if="currentFormat.pluginId === 'html'" class="h-full flex flex-col relative group">
               <button 
                  @click="copyTextWithToast(currentFormat.data)" 
                  class="absolute right-4 top-4 z-10 bg-white/90 border border-gray-200 shadow-sm px-2 py-1 rounded text-xs hover:bg-gray-100 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1"
                >
                  <PhCopy :size="12" /> Copy
                </button>
              <iframe 
                :srcdoc="currentFormat.data" 
                class="w-full h-full border-none bg-white" 
                sandbox="allow-same-origin"
              ></iframe>
            </div>

            <!-- Files -->
             <div v-else-if="currentFormat.pluginId === 'files'" class="p-4">
               <ul class="space-y-2">
                 <li v-for="(file, i) in getFiles(currentFormat)" :key="i">
                   <button 
                    @click="copyTextWithToast(file.path)"
                    class="w-full text-left group flex items-center gap-3 p-3 rounded-lg border border-gray-100 hover:border-blue-200 hover:bg-blue-50 transition-all"
                    :title="file.path"
                   >
                     <div class="p-2 bg-gray-100 rounded-md text-gray-500 group-hover:bg-white group-hover:text-blue-500">
                        <PhFileIcon :size="20" />
                     </div>
                     <div class="flex-1 min-w-0">
                       <div class="text-sm font-medium text-gray-700 group-hover:text-blue-700 truncate">
                         {{ file.name || file.path }}
                       </div>
                       <div class="text-xs text-gray-400 flex items-center gap-2">
                          <span v-if="file.size">{{ formatBytes(file.size) }}</span>
                          <span v-if="file.mime" class="px-1.5 py-0.5 bg-gray-100 rounded text-[10px] uppercase">{{ file.mime }}</span>
                          <span class="text-gray-300 truncate">{{ file.path }}</span>
                       </div>
                     </div>
                     <PhCopy :size="16" class="text-gray-300 group-hover:text-blue-400 opacity-0 group-hover:opacity-100" />
                   </button>
                 </li>
               </ul>
             </div>

            <!-- Text/Code -->
            <div v-else class="relative group min-h-full">
              <button 
                @click="copyTextWithToast(getFormatText(currentFormat))" 
                class="absolute right-4 top-4 bg-white/90 border border-gray-200 shadow-sm px-2 py-1 rounded text-xs hover:bg-gray-100 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1"
              >
                <PhCopy :size="12" /> Copy
              </button>
              <pre class="p-6 font-mono text-xs leading-relaxed whitespace-pre-wrap break-words text-gray-800">{{ getFormatText(currentFormat) }}</pre>
            </div>
          </div>
          
          <div class="bg-gray-50 border-t border-gray-100 px-4 py-2 text-[10px] text-gray-500 flex gap-4 font-mono flex-shrink-0">
            <div>KIND: {{ currentFormat.kind || 'unknown' }}</div>
            <div>PRIORITY: {{ currentFormat.priority || 0 }}</div>
            <div v-if="currentFormat.pluginId === 'image' && currentFormat.metadata?.width">
              DIMENSIONS: {{ currentFormat.metadata.width }} x {{ currentFormat.metadata.height }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, watch, onUnmounted, ref } from 'vue'
import { PhMouseSimple, PhTextT, PhImage as PhImageIcon, PhFile as PhFileIcon, PhCube, PhCopy, PhTrash } from '@phosphor-icons/vue'

const props = defineProps({
  item: Object,
  fullData: Object,
  loading: Boolean,
  activeFormatIndex: Number
})

const emit = defineEmits(['format-change', 'copy', 'delete', 'toast'])

const imageBlobUrl = ref(null)

const currentFormat = computed(() => props.fullData?.formats?.[props.activeFormatIndex])

watch(currentFormat, (newFormat, oldFormat) => {
  if (oldFormat?.pluginId === 'image' && imageBlobUrl.value) {
    URL.revokeObjectURL(imageBlobUrl.value)
    imageBlobUrl.value = null
  }
  
  if (newFormat?.pluginId === 'image') {
    const dataUrl = getImageSrc(newFormat)
    if (dataUrl) {
      fetch(dataUrl)
        .then(res => res.blob())
        .then(blob => {
          imageBlobUrl.value = URL.createObjectURL(blob)
        })
        .catch(() => {
          imageBlobUrl.value = null
        })
    }
  }
}, { immediate: true })

onUnmounted(() => {
  if (imageBlobUrl.value) {
    URL.revokeObjectURL(imageBlobUrl.value)
  }
})

const formatBytes = (bytes) => {
  if (!bytes && bytes !== 0) return ''
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

const getFormatSize = (fmt) => {
  if (fmt.metadata?.byteSize) return formatBytes(fmt.metadata.byteSize)
  if (fmt.metadata?.size) return formatBytes(fmt.metadata.size)
  if (typeof fmt.data === 'string') return formatBytes(fmt.data.length)
  return '?'
}

const getImageSrc = (fmt) => {
  if (!fmt?.data) return ''
  if (typeof fmt.data === 'string') {
    return fmt.data.startsWith('data:') ? fmt.data : `data:image/png;base64,${fmt.data}`
  }
  return ''
}

const getFormatText = (fmt) => {
  if (!fmt) return ''
  return typeof fmt.data === 'string' ? fmt.data : JSON.stringify(fmt.data, null, 2)
}

const getFiles = (fmt) => {
  if (Array.isArray(fmt.data)) {
    return fmt.data.map(item => {
      if (typeof item === 'string') {
        return { path: item, name: item.split('/').pop() || item }
      }
      return {
          path: item.source_path || item.sourcePath || item.path,
          name: item.name || (item.source_path || item.sourcePath || item.path).split('/').pop(),
          size: item.size,
          mime: item.mime
      }
    })
  }
  return []
}

const copyTextWithToast = (text) => {
  navigator.clipboard.writeText(text).then(() => {
    emit('toast', { title: 'Copied', message: 'Content copied to clipboard', type: 'success' })
  })
}

const openImageInNewTab = () => {
  if (imageBlobUrl.value) {
    window.open(imageBlobUrl.value, '_blank')
  }
}
</script>

<style scoped>
.btn-icon {
  @apply p-2 rounded-lg transition-all flex items-center justify-center shadow-sm;
}

.btn-icon:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.btn-icon:active {
  transform: translateY(0);
}

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
