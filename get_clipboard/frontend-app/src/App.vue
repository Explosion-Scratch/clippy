<template>
  <div class="flex w-full h-screen overflow-hidden bg-white text-gray-900 relative">
    <!-- Mobile Sidebar Overlay -->
    <div 
      v-if="showMobileSidebar && isMobile" 
      class="absolute inset-0 bg-black/50 z-40"
      @click="showMobileSidebar = false"
    />

    <Sidebar 
      class="transition-transform duration-300 absolute md:relative z-50 h-full"
      :class="[
        isMobile ? (showMobileSidebar ? 'translate-x-0' : '-translate-x-full') : 'translate-x-0'
      ]"
      :stats="stats"
      :current-filter="currentFilter"
      :connected="connected"
      @filter="setFilter"
      @action="handleAction"
    />
    
    <main class="flex-1 flex flex-col h-full min-w-0 relative">
      <TopBar 
        ref="topBar"
        v-model:search-query="searchQuery"
        :is-searching="isSearching"
        :sort-by="sortBy"
        :sort-direction="sortDirection"
        :selected-types="selectedTypes"
        :show-hamburger="isMobile"
        @refresh="refreshAll"
        @sort="setSortBy"
        @toggle-type="toggleType"
        @toggle-sidebar="toggleMobileSidebar"
      />
      
      <div class="flex-1 flex overflow-hidden relative">
        <ItemList 
          class="transition-all duration-300 absolute md:relative w-full md:w-auto h-full z-10"
          :class="[
            isMobile && selectedItem ? '-translate-x-full opacity-0 pointer-events-none' : 'translate-x-0 opacity-100'
          ]"
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
          @toast="(payload) => typeof payload === 'object' ? showToast(payload.title, payload.message, payload.type) : showToast(payload)"
          @toggle-select="toggleSelect"
          @multi-select="multiSelect"
        />
        
        <ItemDetail 
          class="transition-all duration-300 absolute md:relative w-full h-full bg-white z-20"
          :class="[
            isMobile ? (selectedItem ? 'translate-x-0' : 'translate-x-full') : 'translate-x-0'
          ]"
          :item="selectedItem"
          :full-data="fullItemData"
          :loading="loadingDetails"
          :active-format-index="activeFormatIndex"
          :show-back-button="isMobile"
          @format-change="activeFormatIndex = $event"
          @copy="copyItem"
          @delete="deleteItem"
          @toast="(payload) => typeof payload === 'object' ? showToast(payload.title, payload.message, payload.type) : showToast(payload)"
          @back="handleBackToList"
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
import { onMounted, onUnmounted, watch, computed, ref } from 'vue'
import Sidebar from './components/Sidebar.vue'
import TopBar from './components/TopBar.vue'
import ItemList from './components/ItemList.vue'
import ItemDetail from './components/ItemDetail.vue'
import StatsModal from './components/StatsModal.vue'
import ImportModal from './components/ImportModal.vue'
import SettingsModal from './components/SettingsModal.vue'
import ToastContainer from './components/ToastContainer.vue'
import { useClipboard } from './composables/useClipboard'
import { PhList } from '@phosphor-icons/vue'

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
  selectedTypes,
  sortBy,
  sortDirection,
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
  toggleType,
  setSortBy,
  refreshAll,
  copyToClipboard,
  loadItemById,
  handleImport,
  updateDataDir,
  handleAction,
  showToast,
  toggleSelect,
  multiSelect,
} = useClipboard()

// Responsive state
const showMobileSidebar = ref(false)
const isMobile = ref(window.innerWidth < 768)

// State Management
const updateUrl = () => {
  const params = new URLSearchParams()
  
  if (selectedItem.value) params.set('item', selectedItem.value.id)
  if (searchQuery.value) params.set('q', searchQuery.value)
  if (selectedTypes.value.size > 0) params.set('filter', Array.from(selectedTypes.value).join(','))
  if (sortBy.value !== 'date') params.set('sort', sortBy.value)
  if (sortDirection.value !== 'desc') params.set('order', sortDirection.value)
  if (showStatsModal.value) params.set('modal', 'stats')
  if (showImportModal.value) params.set('modal', 'import')
  if (showSettingsModal.value) params.set('modal', 'settings')
  
  // Update URL without reloading
  const newUrl = `${window.location.pathname}${params.toString() ? '?' + params.toString() : ''}`
  window.history.replaceState({}, '', newUrl)
}

const restoreState = async () => {
  const params = new URLSearchParams(window.location.search)
  
  if (params.has('q')) searchQuery.value = params.get('q')
  if (params.has('filter')) {
    const filters = params.get('filter').split(',').filter(Boolean)
    selectedTypes.value.clear()
    filters.forEach(f => selectedTypes.value.add(f))
  }
  if (params.has('sort')) sortBy.value = params.get('sort')
  if (params.has('order')) sortDirection.value = params.get('order')
  
  const modal = params.get('modal')
  if (modal === 'stats') showStatsModal.value = true
  if (modal === 'import') showImportModal.value = true
  if (modal === 'settings') showSettingsModal.value = true
  
  const itemId = params.get('item')
  if (itemId) {
    // We need to wait for items to load or load specific item
    // For now, let's try to load it specifically if not in list
    await loadItemById(itemId)
  }
}

// Watchers for state sync
watch([selectedItem, searchQuery, () => selectedTypes.value.size, sortBy, sortDirection, showStatsModal, showImportModal, showSettingsModal], () => {
  updateUrl()
})

// Responsive handlers
const handleResize = () => {
  isMobile.value = window.innerWidth < 768
  if (!isMobile.value) showMobileSidebar.value = false
}

const toggleMobileSidebar = () => {
  showMobileSidebar.value = !showMobileSidebar.value
}

const handleBackToList = () => {
  selectItem(null)
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  window.addEventListener('resize', handleResize)
  window.addEventListener('popstate', restoreState)
  
  await restoreState()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('popstate', restoreState)
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
