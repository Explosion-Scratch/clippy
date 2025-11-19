<template>
  <div class="h-14 border-b border-gray-200 flex items-center px-4 gap-4 bg-white z-10 shadow-sm">
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
</template>

<script setup>
import { ref } from 'vue'
import { PhMagnifyingGlass, PhArrowsClockwise } from '@phosphor-icons/vue'

defineProps({
  searchQuery: String,
  isSearching: Boolean
})

defineEmits(['update:searchQuery', 'refresh'])

const searchInputRef = ref(null)

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

