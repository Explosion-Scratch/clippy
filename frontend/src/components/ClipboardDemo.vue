<script setup>
import { ref, computed, watch } from 'vue'
import ClipboardItem from './ClipboardItem.vue'
import PreviewWindow from './PreviewWindow.vue'
import demoItems from '../data/demo-items.json'
import demoPreviews from '../data/demo-previews.json'

const searchQuery = ref('')
const selectedIndex = ref(0)

const items = ref(demoItems.map((item) => ({
  ...item,
  text: getTextContent(item),
  timestamp: new Date(item.date).getTime(),
  copies: item.copyCount || 0,
  preview: item.summary,
  files: getFilesContent(item)
})))

function getTextContent(item) {
  const textFormat = item.formats?.find(f => f.id === 'text')
  return textFormat?.data || item.summary
}

function getFilesContent(item) {
  const filesFormat = item.formats?.find(f => f.id === 'files')
  return filesFormat?.data?.files || null
}

const filteredItems = computed(() => {
  if (!searchQuery.value.trim()) {
    return items.value
  }
  const query = searchQuery.value.toLowerCase()
  return items.value.filter(item => {
    if (item.text && item.text.toLowerCase().includes(query)) return true
    if (item.summary && item.summary.toLowerCase().includes(query)) return true
    if (item.files && item.files.some(f => f.toLowerCase().includes(query))) return true
    return false
  })
})

const selectedItem = computed(() => {
  if (selectedIndex.value >= 0 && filteredItems.value[selectedIndex.value]) {
    return filteredItems.value[selectedIndex.value]
  }
  return null
})

const selectedPreview = computed(() => {
  if (!selectedItem.value) return null
  return demoPreviews[selectedItem.value.id] || null
})

const totalItems = computed(() => items.value.length)

function handleKeyDown(e) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (selectedIndex.value < filteredItems.value.length - 1) {
      selectedIndex.value++
    }
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (selectedIndex.value > 0) {
      selectedIndex.value--
    }
  }
}

function selectItem(index) {
  selectedIndex.value = index
}

watch(searchQuery, () => {
  selectedIndex.value = 0
})
</script>

<template>
  <section id="demo" class="section clipboard-demo-section">
    <div class="container">
      <div class="section-title">
        <h2>Interactive Demo</h2>
        <p>Experience the clipboard manager. Search, navigate with arrow keys, and preview your clipboard history.</p>
      </div>

      <div class="demo-wrapper">
        <div class="clipboard-demo" @keydown="handleKeyDown" tabindex="0">
          <div class="search-container">
            <input 
              v-model="searchQuery"
              type="text"
              :placeholder="`Search ${totalItems} items`"
              class="search-input"
              autocomplete="off"
              autocapitalize="off"
              autocorrect="off"
              spellcheck="false"
            />
          </div>

          <div class="content-area">
            <div class="items-container">
              <div v-if="filteredItems.length === 0" class="empty-state">
                <div class="empty-icon">🔍</div>
                <p>{{ searchQuery ? 'No results' : 'Copy something to get started' }}</p>
              </div>
              <div v-else class="clipboard-list">
                <ClipboardItem 
                  v-for="(item, idx) in filteredItems"
                  :key="item.id"
                  :item="{ ...item, index: idx }"
                  :selected="idx === selectedIndex"
                  @mouseenter="selectItem(idx)"
                />
              </div>
            </div>
          </div>

          <div v-if="selectedItem" class="status-bar">
            <div class="status-item">
              <span class="status-value">{{ new Date(selectedItem.timestamp).toLocaleString('en-US', { month: '2-digit', day: '2-digit', hour: 'numeric', minute: '2-digit' }) }}</span>
            </div>
            <div class="status-item">
              <span class="status-value">{{ selectedItem.copies }} copies</span>
            </div>
          </div>
        </div>

        <PreviewWindow :item="selectedItem" :preview="selectedPreview" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.clipboard-demo-section {
  background: var(--bg-secondary);
}

.demo-wrapper {
  display: flex;
  gap: 0;
  justify-content: center;
  align-items: flex-start;
  flex-wrap: wrap;
}

.clipboard-demo {
  width: 400px;
  background: var(--bg-primary);
  border-radius: 12px 0 0 12px;
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  outline: none;
  display: flex;
  flex-direction: column;
  font-family: system-ui, sans-serif;
  min-height: 400px;
}

.clipboard-demo:focus {
  box-shadow: var(--shadow-lg), 0 0 0 2px var(--accent);
}

.search-container {
  padding: 8px;
  margin-top: 3px;
}

.search-input {
  width: 100%;
  padding: 5px 8px;
  border: 0.5px solid var(--border-light);
  border-radius: 5px;
  font-family: system-ui, sans-serif;
  font-size: 0.9rem;
  background: var(--bg-primary);
  color: var(--text-primary);
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
  transition: all var(--transition-fast);
}

.search-input::placeholder {
  color: var(--text-muted);
  opacity: 0.7;
}

.search-input:focus {
  outline: none;
  border-color: transparent;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.2);
}

.content-area {
  display: flex;
  flex: 1;
  min-height: 0;
}

.items-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow-y: auto;
  max-height: 300px;
}

.clipboard-list {
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-muted);
}

.empty-icon {
  font-size: 32px;
  filter: grayscale(0.3);
  margin-bottom: 8px;
}

.status-bar {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  background: var(--bg-tertiary);
  color: var(--text-muted);
  border-radius: 4px;
  font-size: 0.75em;
  margin: auto 4px 4px 4px;
  height: 20px;
  line-height: 20px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  justify-content: center;
}

.status-value {
  font-weight: 300;
  color: var(--text-secondary);
}

@media (max-width: 900px) {
  .demo-wrapper {
    flex-direction: column;
    align-items: center;
  }

  .clipboard-demo {
    width: 100%;
    max-width: 400px;
    border-radius: 12px 12px 0 0;
  }
}
</style>
