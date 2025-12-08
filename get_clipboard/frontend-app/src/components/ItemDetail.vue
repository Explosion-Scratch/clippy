<template>
  <div class="flex-[2] bg-white flex flex-col h-full overflow-hidden relative">
    <div v-if="!item" class="absolute inset-0 flex flex-col items-center justify-center text-gray-300 select-none">
      <PhMouseSimple :size="64" class="mb-4" />
      <p class="text-sm text-gray-400">Select an item to view details</p>
    </div>

    <div v-else class="flex flex-col h-full">
      <div class="px-6 py-4 border-b border-gray-200 bg-white flex-shrink-0">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-3">
            <button 
              v-if="showBackButton"
              @click="$emit('back')"
              class="p-2 -ml-2 hover:bg-gray-100 rounded-lg text-gray-600"
            >
              <PhArrowLeft :size="20" />
            </button>
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
              <div class="text-xs text-gray-500 flex items-center gap-2 flex-wrap">
                <span>{{ new Date(item.date).toLocaleString() }}</span>
                <span v-if="item.firstDate">•</span>
                <span v-if="item.firstDate">First: {{ new Date(item.firstDate).toLocaleString() }}</span>
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
              v-if="!isEditing && isEditable"
              @click="startEdit" 
              class="btn-icon bg-stone-500/10 hover:bg-stone-500/10 transition-colors duration-200 text-slate-900"
              title="Edit Item"
            >
              <PhPencil :size="18" />
            </button>
            <button 
              @click="$emit('copy', item.id)" 
              class="btn-icon bg-stone-500/10 hover:bg-stone-500/10 transition-colors duration-200 text-slate-900"
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

        <div v-if="previewData?.formatsOrder?.length > 0" class="flex items-end gap-1 overflow-x-auto scrollbar-thin">
          <button 
            v-for="format in previewData.formatsOrder" 
            :key="format"
            @click="activeTab = format"
            class="px-4 py-2 text-xs font-medium rounded-t-lg border-t border-x transition-all capitalize"
            :class="activeTab === format 
              ? 'bg-white border-gray-200 text-gray-900 relative top-[1px] shadow-sm' 
              : 'border-transparent text-gray-500 hover:bg-gray-50'"
          >
            {{ format }}
          </button>
        </div>
      </div>

      <div class="flex-1 bg-gray-50 p-6 min-h-0 flex flex-col">
        <div v-if="loading" class="flex items-center justify-center h-full">
          <div class="spinner w-8 h-8" />
        </div>
        
        <div v-else-if="item" class="flex-1 bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden flex flex-col max-w-4xl mx-auto w-full relative group">

          <div v-if="isEditing" class="flex-1 flex flex-col p-4">
            <textarea 
              v-model="editedText"
              class="flex-1 w-full p-3 border border-gray-200 rounded-lg font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
            ></textarea>
            <div class="flex justify-end gap-2 mt-4">
              <button @click="cancelEdit" class="px-4 py-2 text-sm bg-gray-100 rounded-lg hover:bg-gray-200">Cancel</button>
              <button @click="saveEdit" class="px-4 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600">Save</button>
            </div>
          </div>

          <template v-else>
            <div class="flex-1 relative bg-white" @dblclick="handlePreviewDblClick">
               <div v-if="previewLoading" class="absolute inset-0 flex items-center justify-center bg-white z-10">
                <div class="spinner w-8 h-8" />
              </div>
              
              <div v-if="error" class="absolute inset-0 flex items-center justify-center text-red-500 p-4 text-center">
                <p>Error loading preview: {{ error }}</p>
              </div>

              <iframe 
                v-if="previewData && activeTab && previewData.data[activeTab]"
                :key="activeTab"
                :srcdoc="getEnhancedHtml(previewData.data[activeTab].html)"
                ref="previewFrame"
                class="w-full h-full border-none bg-white block" 
                sandbox="allow-same-origin allow-scripts"
              ></iframe>
            </div>

            <div class="absolute right-4 top-4 opacity-0 group-hover:opacity-100 transition-opacity z-20">
              <button 
                @click="copyCurrentContent" 
                class="bg-white/90 border border-gray-200 shadow-sm px-3 py-1.5 rounded-lg text-xs hover:bg-gray-100 flex items-center gap-2 font-medium text-gray-700"
              >
                <PhCopy :size="14" /> 
                {{ activeTab === 'image' ? 'Copy Image' : 'Copy Content' }}
              </button>
            </div>
            
            <div class="bg-gray-50 border-t border-gray-100 px-4 py-2 text-[10px] text-gray-500 flex gap-4 font-mono flex-shrink-0">
              <div v-if="previewData?.kind">KIND: {{ previewData.kind }}</div>
              <div v-if="activeTab">FORMAT: {{ activeTab }}</div>
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, onMounted, computed } from 'vue'
import { PhMouseSimple, PhTextT, PhImage as PhImageIcon, PhFile as PhFileIcon, PhCube, PhCopy, PhTrash, PhArrowLeft, PhPencil } from '@phosphor-icons/vue'

const props = defineProps({
  item: Object,
  fullData: Object,
  loading: Boolean,
  activeFormatIndex: Number,
  showBackButton: Boolean
})

const emit = defineEmits(['format-change', 'copy', 'delete', 'toast', 'back', 'refresh'])

const previewData = ref(null)
const previewLoading = ref(false)
const activeTab = ref('')
const error = ref(null)
const isEditing = ref(false)
const editedText = ref('')
const previewFrame = ref(null)

const isEditable = computed(() => {
  if (!previewData.value || !activeTab.value) return false
  const kind = previewData.value.kind
  return kind !== 'file' && kind !== 'image'
})

const fetchPreview = async (id) => {
  if (!id) return
  previewLoading.value = true
  error.value = null
  previewData.value = null
  
  try {
    const res = await fetch(`/item/${id}/preview`)
    if (!res.ok) throw new Error('Failed to load preview')
    const data = await res.json()
    previewData.value = data
    
    // Set initial active tab
    if (data.formatsOrder && data.formatsOrder.length > 0) {
      activeTab.value = data.formatsOrder[0]
    }
  } catch (err) {
    console.error('Preview fetch error:', err)
    error.value = err.message
  } finally {
    previewLoading.value = false
  }
}

watch(() => props.item?.id, (newId) => {
  if (newId) {
    fetchPreview(newId)
  } else {
    previewData.value = null
    activeTab.value = ''
  }
}, { immediate: true })

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

const copyTextWithToast = (text) => {
  navigator.clipboard.writeText(text).then(() => {
    emit('toast', { title: 'Copied', message: 'Content copied to clipboard', type: 'success' })
  })
}

const copyImage = async () => {
  const html = previewData.value?.data?.image?.html
  if (!html) return

  // Extract base64 from HTML
  const match = html.match(/src="(data:image\/[^;]+;base64,[^"]+)"/)
  if (match && match[1]) {
    try {
      const res = await fetch(match[1])
      const blob = await res.blob()
      await navigator.clipboard.write([
        new ClipboardItem({
          [blob.type]: blob
        })
      ])
      emit('toast', { title: 'Copied', message: 'Image copied to clipboard', type: 'success' })
    } catch (err) {
      console.error('Failed to copy image:', err)
      emit('toast', { title: 'Error', message: 'Failed to copy image', type: 'error' })
    }
  }
}

const copyCurrentContent = () => {
  if (activeTab.value === 'image') {
    copyImage()
    return
  }
  
  const text = previewData.value?.data?.[activeTab.value]?.text
  if (text) {
    copyTextWithToast(text)
  } else {
    emit('copy', props.item.id)
  }
}

const startEdit = () => {
  const formatData = previewData.value?.data?.[activeTab.value]
  const text = formatData?.text || ''
  editedText.value = text
  isEditing.value = true
}

const cancelEdit = () => {
  isEditing.value = false
  editedText.value = ''
}

const saveEdit = async () => {
  if (!props.item?.id) return
  try {
    const payload = {}
    payload[activeTab.value] = editedText.value
    
    const res = await fetch(`/item/${props.item.id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    if (!res.ok) throw new Error('Failed to save edit')
    isEditing.value = false
    emit('toast', { title: 'Saved', message: `${activeTab.value} format updated successfully`, type: 'success' })
    await fetchPreview(props.item.id)
    emit('refresh')
  } catch (err) {
    console.error('Failed to save edit:', err)
    emit('toast', { title: 'Error', message: 'Failed to save edit', type: 'error' })
  }
}

const handlePreviewDblClick = () => {
  if (isEditable.value) {
    startEdit()
  }
}

const getEnhancedHtml = (html) => {
  if (!html) return html
  const script = `<script>
    document.addEventListener('dblclick', (e) => {
      e.preventDefault();
      parent.postMessage({ type: 'preview-dblclick' }, '*');
    });
  <\\/script>`
  return html.includes('</body>') ? html.replace('</body>', `${script}</body>`) : `${html}${script}`
}

const handleMessage = (event) => {
  if (event.data?.type === 'copy') {
    navigator.clipboard.writeText(event.data.text).catch(console.error)
  }
  if (event.data?.type === 'toast') {
    emit('toast', event.data.toast || { message: event.data.message, type: event.data.level || 'info' })
  }
  if (event.data?.type === 'preview-dblclick') {
    handlePreviewDblClick()
  }
}

onMounted(() => {
  window.addEventListener('message', handleMessage)
})

import { onUnmounted } from 'vue'
onUnmounted(() => {
  window.removeEventListener('message', handleMessage)
})
</script>

<style scoped>
.btn-icon {
  @apply p-2 rounded-lg transition-all flex items-center justify-center shadow-sm;
}

.btn-icon:hover {
  transform: translateY(-1px);
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
