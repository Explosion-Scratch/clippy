import { ref, watch } from 'vue'

export function useClipboard() {
  const API_BASE = window.location.origin
  const LIMIT = 50

  const items = ref([])
  const stats = ref(null)
  const selectedItem = ref(null)
  const fullItemData = ref(null)
  const loading = ref(false)
  const loadingMore = ref(false)
  const loadingDetails = ref(false)
  const searchQuery = ref('')
  const isSearching = ref(false)
  const currentFilter = ref('all')
  const sortBy = ref('date')
  const sortDirection = ref('desc')
  const connected = ref(true)
  const dataDir = ref('')
  const offset = ref(0)
  const endReached = ref(false)
  const activeFormatIndex = ref(0)
  const selectedIds = ref(new Set())
  const showStatsModal = ref(false)
  const showImportModal = ref(false)
  const showSettingsModal = ref(false)
  const toasts = ref([])
  const topBar = ref(null)

  let searchTimeout = null

  const showToast = (title, message = '', type = 'success') => {
    const id = Date.now()
    toasts.value.push({ id, title, message, type })
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== id)
    }, 3000)
  }

  const copyToClipboard = (text) => {
    navigator.clipboard.writeText(String(text)).then(() => {
      showToast('Copied', String(text))
    })
  }

  const fetchStats = async () => {
    try {
      const res = await fetch(`${API_BASE}/stats`)
      if (res.ok) {
        stats.value = await res.json()
      }
    } catch (e) {
      connected.value = false
    }
  }

  const fetchDir = async () => {
    try {
      const res = await fetch(`${API_BASE}/dir`)
      if (res.ok) {
        const data = await res.json()
        dataDir.value = data.path
      }
    } catch (e) {}
  }

  const loadItems = async (reset = false) => {
    if (loading.value || (endReached.value && !reset)) return
    
    if (reset) {
      items.value = []
      offset.value = 0
      endReached.value = false
      loading.value = true
    } else {
      loadingMore.value = true
    }

    try {
      const hasFilter = currentFilter.value !== 'all'
      const hasSearch = !!searchQuery.value
      const hasSort = sortBy.value !== 'date' || sortDirection.value !== 'desc'
      
      let url
      if (hasSearch || hasFilter) {
        const params = new URLSearchParams()
        params.append('offset', offset.value)
        params.append('count', LIMIT)
        
        if (hasSearch) {
            params.append('query', searchQuery.value)
        }
        
        if (hasFilter) {
            params.append('formats', currentFilter.value)
        }
        
        params.append('sort', sortBy.value)
        
        url = `${API_BASE}/search?${params.toString()}`
      } else {
        const params = new URLSearchParams()
        params.append('offset', offset.value)
        params.append('count', LIMIT)
        if (hasSort) {
          params.append('sort', sortBy.value)
        }
        url = `${API_BASE}/items?${params.toString()}`
      }
      
      const res = await fetch(url)
      if (!res.ok) throw new Error('Failed to fetch')
      let newItems = await res.json()
      
      if (sortDirection.value === 'asc' && hasSort) {
        newItems = newItems.reverse()
      }
      
      if (newItems.length < LIMIT) endReached.value = true
      
      items.value = reset ? newItems : [...items.value, ...newItems]
      offset.value += LIMIT
      connected.value = true
    } catch (err) {
      showToast('Error', 'Connection lost', 'error')
      connected.value = false
    } finally {
      loading.value = false
      loadingMore.value = false
    }
  }

  const loadItemById = async (id) => {
    const existing = items.value.find(i => i.id === id)
    if (existing) {
      selectItem(existing)
    } else {
      try {
        const res = await fetch(`${API_BASE}/item/${id}`)
        if (res.ok) {
          const item = await res.json()
          items.value = [item, ...items.value]
          selectItem(item)
        }
      } catch (e) {
        showToast('Error', 'Item not found', 'error')
      }
    }
  }

  const loadFullItem = async (id) => {
    if (!id) return
    loadingDetails.value = true
    try {
      const res = await fetch(`${API_BASE}/item/${id}/data`)
      if (!res.ok) throw new Error('Failed')
      fullItemData.value = await res.json()
      activeFormatIndex.value = 0
    } catch (err) {
      showToast('Error', 'Failed to load details', 'error')
    } finally {
      loadingDetails.value = false
    }
  }

  const updateDataDir = async (mode, path) => {
    try {
      const res = await fetch(`${API_BASE}/dir`, {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({ mode, path })
      })
      if (!res.ok) throw new Error('Failed')
      const data = await res.json()
      dataDir.value = data.path
      showToast('Success', 'Data directory updated')
      showSettingsModal.value = false
      refreshAll()
    } catch (e) {
      showToast('Error', 'Failed to update directory', 'error')
    }
  }

  const selectItem = (item, event) => {
    if (event && (event.metaKey || event.ctrlKey)) {
      selectedIds.value.has(item.id) 
        ? selectedIds.value.delete(item.id) 
        : selectedIds.value.add(item.id)
      return
    } else if (event && event.shiftKey && selectedItem.value) {
      const idx1 = items.value.findIndex(i => i.id === selectedItem.value.id)
      const idx2 = items.value.findIndex(i => i.id === item.id)
      const start = Math.min(idx1, idx2)
      const end = Math.max(idx1, idx2)
      items.value.slice(start, end + 1).forEach(i => selectedIds.value.add(i.id))
      return
    }
    selectedIds.value.clear()
    selectedItem.value = item
    loadFullItem(item.id)
  }

  const copyItem = async (id) => {
    await fetch(`${API_BASE}/item/${id}/copy`, { method: 'POST' })
    showToast('Copied', 'Item copied to clipboard')
  }

  const deleteItem = async (id) => {
    if (!confirm('Permanently delete this item?')) return
    await fetch(`${API_BASE}/item/${id}`, { method: 'DELETE' })
    items.value = items.value.filter(i => i.id !== id)
    if (selectedItem.value?.id === id) {
      selectedItem.value = null
      fullItemData.value = null
    }
    showToast('Deleted', 'Item removed')
    fetchStats()
  }

  const deleteSelected = async () => {
    if (!confirm(`Delete ${selectedIds.value.size} items?`)) return
    for (const id of selectedIds.value) {
      await fetch(`${API_BASE}/item/${id}`, { method: 'DELETE' })
    }
    items.value = items.value.filter(i => !selectedIds.value.has(i.id))
    selectedIds.value.clear()
    selectedItem.value = null
    showToast('Deleted', 'Items removed')
    fetchStats()
  }

  const clearSelection = () => selectedIds.value.clear()

  const toggleSelect = (id) => {
    selectedIds.value.has(id) 
      ? selectedIds.value.delete(id) 
      : selectedIds.value.add(id)
  }

  const multiSelect = (ids) => {
    ids.forEach(id => {
      if (!selectedIds.value.has(id)) {
        selectedIds.value.add(id)
      }
    })
  }

  const handleScroll = (e) => {
    if (e.target.scrollTop + e.target.clientHeight >= e.target.scrollHeight - 50) {
      loadItems()
    }
  }

  const setFilter = (filter) => {
    currentFilter.value = filter
    loadItems(true)
  }

  const setSortBy = (sort) => {
    if (sortBy.value === sort) {
      sortDirection.value = sortDirection.value === 'desc' ? 'asc' : 'desc'
    } else {
      sortBy.value = sort
      sortDirection.value = 'desc'
    }
    loadItems(true)
  }

  const refreshAll = () => {
    loadItems(true)
    fetchStats()
  }

  const handleImport = async (data) => {
    try {
      const itemsToImport = Array.isArray(data) ? data : [data]
      let count = 0
      for (const item of itemsToImport) {
        await fetch(`${API_BASE}/save`, {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify(item)
        })
        count++
      }
      showToast('Imported', `${count} items imported`)
      refreshAll()
      showImportModal.value = false
    } catch (e) {
      showToast('Error', 'Import failed', 'error')
    }
  }

  const exportAll = async () => {
    try {
      const exportData = items.value
      const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `clippith-export-${new Date().toISOString()}.json`
      a.click()
      URL.revokeObjectURL(url)
      showToast('Exported', `${items.value.length} items exported`)
    } catch (e) {
      showToast('Error', 'Export failed', 'error')
    }
  }

  const handleAction = (action) => {
    switch (action) {
      case 'import':
        showImportModal.value = true
        break
      case 'export':
        exportAll()
        break
      case 'stats':
        showStatsModal.value = true
        fetchStats()
        break
      case 'settings':
        showSettingsModal.value = true
        break
    }
  }

  watch(searchQuery, () => {
    isSearching.value = true
    clearTimeout(searchTimeout)
    searchTimeout = setTimeout(() => {
      loadItems(true)
      isSearching.value = false
    }, 300)
  })

  // Initial load
  fetchStats()
  fetchDir()
  loadItems(true)

  // Auto refresh
  setInterval(async () => {
    try {
      const res = await fetch(`${API_BASE}/mtime`)
      const data = await res.json()
      if (items.value.length > 0 && data.id && data.id !== items.value[0].hash && offset.value < LIMIT && !searchQuery.value) {
        loadItems(true)
        fetchStats()
      }
    } catch(e) {}
  }, 2000)

  return {
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
    toggleSelect,
    multiSelect,
    handleScroll,
    setFilter,
    setSortBy,
    refreshAll,
    copyToClipboard,
    loadItemById,
    handleImport,
    updateDataDir,
    handleAction,
    showToast,
  }
}

