<template>
  <aside class="w-64 bg-[#f7f7f5] border-r border-gray-200 flex flex-col h-full">
    <div class="p-4 flex items-center gap-3 select-none cursor-default mb-2">
      <div class="w-8 h-8 bg-gradient-to-br from-gray-900 to-gray-700 text-white rounded-lg flex items-center justify-center text-lg font-bold shadow-md">
        C
      </div>
      <span class="font-semibold text-base">Clippith</span>
    </div>

    <div class="flex-1 overflow-y-auto px-3 py-2 space-y-1 scrollbar-thin">
      <div class="text-xs font-semibold text-gray-400 px-3 py-2 tracking-wider uppercase">
        Library
      </div>
      
      <button
        v-for="filter in filters"
        :key="filter.id"
        @click="$emit('filter', filter.id)"
        class="sidebar-item w-full"
        :class="{ active: currentFilter === filter.id }"
      >
        <component :is="filter.icon" :size="20" :weight="currentFilter === filter.id ? 'fill' : 'regular'" />
        <span>{{ filter.label }}</span>
        <span v-if="filter.count !== undefined" class="ml-auto text-xs text-gray-400 bg-gray-200/50 px-1.5 py-0.5 rounded">
          {{ filter.count }}
        </span>
      </button>

      <div class="text-xs font-semibold text-gray-400 px-3 py-2 mt-6 tracking-wider uppercase">
        Actions
      </div>
      
      <button
        v-for="action in actions"
        :key="action.id"
        @click="$emit('action', action.id)"
        class="sidebar-item w-full"
      >
        <component :is="action.icon" :size="20" />
        <span>{{ action.label }}</span>
      </button>
    </div>
    
    <div class="p-4 border-t border-gray-200 text-xs text-gray-500 flex items-center gap-2">
      <div 
        class="w-2 h-2 rounded-full transition-colors" 
        :class="connected ? 'bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]' : 'bg-red-500'"
      />
      <span>{{ connected ? 'System Operational' : 'Disconnected' }}</span>
    </div>
  </aside>
</template>

<script setup>
import { computed } from 'vue'
import { 
  PhSquaresFour, 
  PhTextT, 
  PhImage, 
  PhFile, 
  PhCode,
  PhUploadSimple, 
  PhDownloadSimple, 
  PhChartBar, 
  PhGear 
} from '@phosphor-icons/vue'

const props = defineProps({
  stats: Object,
  currentFilter: String,
  connected: Boolean
})

defineEmits(['filter', 'action'])

const filters = computed(() => [
  { 
    id: 'all', 
    label: 'All Items', 
    icon: PhSquaresFour,
    count: props.stats?.totalItems 
  },
  { 
    id: 'text', 
    label: 'Text', 
    icon: PhTextT,
    count: props.stats?.typeCounts?.text || 0 
  },
  { 
    id: 'html', 
    label: 'HTML', 
    icon: PhCode,
    count: props.stats?.typeCounts?.html || 0
  },
  { 
    id: 'image', 
    label: 'Images', 
    icon: PhImage,
    count: props.stats?.typeCounts?.image || 0 
  },
  { 
    id: 'file', 
    label: 'Files', 
    icon: PhFile,
    count: props.stats?.typeCounts?.file || 0 
  },
])

const actions = [
  { id: 'import', label: 'Import', icon: PhUploadSimple },
  { id: 'export', label: 'Export All', icon: PhDownloadSimple },
  { id: 'stats', label: 'Statistics', icon: PhChartBar },
  { id: 'settings', label: 'Settings', icon: PhGear },
]
</script>

<style scoped>
.sidebar-item {
  @apply flex items-center gap-3 px-3 py-2 text-sm text-gray-600 rounded-lg hover:bg-gray-100 cursor-pointer transition-all select-none;
}

.sidebar-item:hover {
  transform: translateX(2px);
}

.sidebar-item.active {
  @apply bg-gradient-to-r from-blue-50 to-transparent text-blue-600 font-medium border-l-2 border-blue-500 pl-[10px];
}
</style>

