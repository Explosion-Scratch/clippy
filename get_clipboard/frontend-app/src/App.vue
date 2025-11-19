<template>
  <div class="flex w-full h-screen overflow-hidden bg-white text-gray-900">
    <Sidebar 
      :stats="stats"
      :current-filter="currentFilter"
      :connected="connected"
      @filter="setFilter"
      @action="handleAction"
    />
    
    <main class="flex-1 flex flex-col h-full min-w-0">
      <TopBar 
        ref="topBar"
        v-model:search-query="searchQuery"
        :is-searching="isSearching"
        @refresh="refreshAll"
      />
      
      <div class="flex-1 flex overflow-hidden">
        <ItemList 
          :items="items"
          :selected-item="selectedItem"
          :selected-ids="selectedIds"
          :loading-more="loadingMore"
          :loading="loading"
          @select="selectItem"
          @copy-index="copyToClipboard"
          @scroll="handleScroll"
          @delete-selected="deleteSelected"
          @clear-selection="clearSelection"
        />
        
        <ItemDetail 
          :item="selectedItem"
          :full-data="fullItemData"
          :loading="loadingDetails"
          :active-format-index="activeFormatIndex"
          @format-change="activeFormatIndex = $event"
          @copy="copyItem"
          @delete="deleteItem"
        />
      </div>
    </main>
    
    <StatsModal 
      v-if="showStatsModal"
      :stats="stats"
      @close="showStatsModal = false"
      @load-item="loadItemById"
    />
    
    <ImportModal 
      v-if="showImportModal"
      @close="showImportModal = false"
      @import="handleImport"
    />
    
    <SettingsModal 
      v-if="showSettingsModal"
      :data-dir="dataDir"
      @close="showSettingsModal = false"
      @update="updateDataDir"
    />
    
    <ToastContainer :toasts="toasts" />
  </div>
</template>

<script setup>
import { onMounted, onUnmounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import TopBar from './components/TopBar.vue'
import ItemList from './components/ItemList.vue'
import ItemDetail from './components/ItemDetail.vue'
import StatsModal from './components/StatsModal.vue'
import ImportModal from './components/ImportModal.vue'
import SettingsModal from './components/SettingsModal.vue'
import ToastContainer from './components/ToastContainer.vue'
import { useClipboard } from './composables/useClipboard'

const {
  items,
  stats,
  selectedItem,
  fullItemData,
  loading,
  loadingMore,
  loadingDetails,
  searchQuery,
  isSearching,
  currentFilter,
  connected,
  dataDir,
  activeFormatIndex,
  selectedIds,
  showStatsModal,
  showImportModal,
  showSettingsModal,
  toasts,
  topBar,
  selectItem,
  copyItem,
  deleteItem,
  deleteSelected,
  clearSelection,
  handleScroll,
  setFilter,
  refreshAll,
  copyToClipboard,
  loadItemById,
  handleImport,
  updateDataDir,
  handleAction,
} = useClipboard()

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

const handleKeydown = (e) => {
  if (e.key === '/' && document.activeElement.tagName !== 'INPUT') {
    e.preventDefault()
    topBar.value?.focusSearch()
  }
  if (e.key === 'Escape') {
    showStatsModal.value = false
    showImportModal.value = false
    showSettingsModal.value = false
    clearSelection()
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (!selectedItem.value && items.value.length > 0) {
      selectItem(items.value[0])
    } else if (selectedItem.value) {
      const idx = items.value.findIndex(i => i.id === selectedItem.value.id)
      if (idx < items.value.length - 1) {
        selectItem(items.value[idx + 1])
      }
    }
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (selectedItem.value) {
      const idx = items.value.findIndex(i => i.id === selectedItem.value.id)
      if (idx > 0) {
        selectItem(items.value[idx - 1])
      }
    }
  }
}
</script>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.scrollbar-thin::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.scrollbar-thin::-webkit-scrollbar-track {
  background: transparent;
}

.scrollbar-thin::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 3px;
}

.scrollbar-thin::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.2);
}
</style>
