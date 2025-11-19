<template>
  <div class="border-b border-gray-200 bg-white z-10 shadow-sm">
    <div class="h-14 flex items-center px-4 gap-4">
      <div class="relative flex-1 group">
        <PhMagnifyingGlass 
          :size="18"
          class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-stone-500 transition-colors"
        />
        <input 
          ref="searchInputRef"
          :value="searchQuery"
          @input="$emit('update:searchQuery', $event.target.value)"
          type="text" 
          placeholder="Search items... (Press '/')" 
          class="w-full pl-10 pr-4 py-2 bg-stone-50 border-2 focus:bg-stone-100 hover:bg-stone-100 border-stone-300/30 rounded-lg text-sm outline-none transition-all"
        />
        <div v-if="isSearching" class="absolute right-3 top-1/2 -translate-y-1/2">
          <div class="spinner w-4 h-4" />
        </div>
      </div>
      
      <button 
        @click="$emit('refresh')"
        class="p-2 hover:bg-gray-100 rounded-lg transition-colors text-gray-600 hover:text-gray-900"
        title="Refresh"
      >
        <PhArrowsClockwise :size="20" />
      </button>
    </div>
    
    <div class="px-4 pt-3 pb-2 flex items-center gap-2 flex-wrap">
      <div class="flex items-center gap-2">
        <span class="text-xs text-gray-500 font-medium">Types:</span>
        <button
          v-for="type in typeOptions"
          :key="type.value"
          @click="$emit('toggleType', type.value)"
          class="px-2.5 py-1 text-xs font-medium rounded-md transition-all"
          :class="selectedTypes.has(type.value)
            ? 'bg-stone-500/10 text-black border border-stone-500' 
            : 'bg-gray-100 text-gray-600 hover:bg-gray-200 border border-transparent'"
        >
          {{ type.label }}
        </button>
      </div>
      
      <div class="flex items-center gap-2 ml-4">
        <span class="text-xs text-gray-500 font-medium">Sort by:</span>
        <button
          v-for="option in sortOptions"
          :key="option.value"
          @click="$emit('sort', option.value)"
          class="px-2.5 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1"
          :class="sortBy === option.value 
            ? 'bg-stone-500/10 text-black border border-stone-500' 
            : 'bg-gray-100 text-gray-600 hover:bg-gray-200 border border-transparent'"
        >
          <span>{{ option.label }}</span>
          <PhCaretUp 
            v-if="sortBy === option.value && sortDirection === 'asc'"
            :size="12"
            class="text-black"
          />
          <PhCaretDown 
            v-else-if="sortBy === option.value && sortDirection === 'desc'"
            :size="12"
            class="text-black"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { PhMagnifyingGlass, PhArrowsClockwise, PhCaretUp, PhCaretDown } from '@phosphor-icons/vue'

const props = defineProps({
  searchQuery: String,
  isSearching: Boolean,
  sortBy: String,
  sortDirection: String,
  selectedTypes: Set
})

defineEmits(['update:searchQuery', 'refresh', 'sort', 'toggleType'])

const searchInputRef = ref(null)

const typeOptions = [
  { value: 'text', label: 'Text' },
  { value: 'files', label: 'Files' },
  { value: 'image', label: 'Images' },
  { value: 'html', label: 'HTML' }
]

const allSortOptions = [
  { value: 'relevance', label: 'Relevance' },
  { value: 'date', label: 'Date' },
  { value: 'copies', label: 'Copies' },
  { value: 'type', label: 'Type' }
]

const sortOptions = computed(() => {
  const hasSearch = !!props.searchQuery
  if (hasSearch) {
    return allSortOptions
  }
  return allSortOptions.filter(opt => opt.value !== 'relevance')
})

defineExpose({
  focusSearch: () => searchInputRef.value?.focus()
})
</script>

<style scoped>
.spinner {
  border: 2px solid rgba(0, 0, 0, 0.1);
  border-left-color: #80736b;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>

