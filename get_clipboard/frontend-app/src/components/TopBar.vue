<template>
  <div class="border-b border-gray-200 bg-white z-10 shadow-sm">
    <div class="h-14 flex items-center px-4 gap-4">
      <div class="relative flex-1 group">
        <PhMagnifyingGlass 
          :size="18"
          class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-blue-500 transition-colors"
        />
        <input 
          ref="searchInputRef"
          :value="searchQuery"
          @input="$emit('update:searchQuery', $event.target.value)"
          type="text" 
          placeholder="Search items... (Press '/')" 
          class="w-full pl-10 pr-4 py-2 bg-gray-50 border border-transparent focus:bg-white focus:border-blue-500 focus:ring-4 focus:ring-blue-100 rounded-lg text-sm outline-none transition-all"
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
    
    <div class="px-4 pt-3 pb-2 flex items-center gap-2">
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
</template>

<script setup>
import { ref, computed } from 'vue'
import { PhMagnifyingGlass, PhArrowsClockwise, PhCaretUp, PhCaretDown } from '@phosphor-icons/vue'

const props = defineProps({
  searchQuery: String,
  isSearching: Boolean,
  sortBy: String,
  sortDirection: String
})

defineEmits(['update:searchQuery', 'refresh', 'sort'])

const searchInputRef = ref(null)

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
  border-left-color: #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>

