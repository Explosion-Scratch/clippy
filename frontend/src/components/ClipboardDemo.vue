<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import ClipboardItem from './ClipboardItem.vue'
import PreviewWindow from './PreviewWindow.vue'
import demoItems from '../data/demo-items.json'
import demoPreviews from '../data/demo-previews.json'

const VISIBLE_ITEMS = 10

const searchQuery = ref('')
const selectedIndex = ref(0)
const scrollOffset = ref(0)
const clipboardListRef = ref(null)

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

const displayedItems = computed(() => {
  const start = scrollOffset.value
  const end = start + VISIBLE_ITEMS
  return filteredItems.value.slice(start, end).map((item, idx) => ({
    ...item,
    actualIndex: start + idx
  }))
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

function adjustScrollOffset() {
  if (selectedIndex.value < scrollOffset.value) {
    scrollOffset.value = selectedIndex.value
  } else if (selectedIndex.value >= scrollOffset.value + VISIBLE_ITEMS) {
    scrollOffset.value = selectedIndex.value - VISIBLE_ITEMS + 1
  }
}

function handleKeyDown(e) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (selectedIndex.value < filteredItems.value.length - 1) {
      selectedIndex.value++
      adjustScrollOffset()
    }
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (selectedIndex.value > 0) {
      selectedIndex.value--
      adjustScrollOffset()
    }
  }
}

function selectItem(actualIndex) {
  selectedIndex.value = actualIndex
}

watch(searchQuery, () => {
  selectedIndex.value = 0
  scrollOffset.value = 0
})

const features = [
  {
    icon: 'ph-lightning',
    title: 'Lightning Fast',
    description: 'Native performance with instant search across your entire clipboard history'
  },
  {
    icon: 'ph-magnifying-glass',
    title: 'Powerful Search',
    description: 'Find any item instantly with regex support and smart filtering'
  },
  {
    icon: 'ph-file-text',
    title: 'Rich Content',
    description: 'Full support for text, images, files, HTML, and RTF content'
  },
  {
    icon: 'ph-terminal-window',
    title: 'CLI Power',
    description: 'Complete command-line interface for scripting and automation'
  }
]
</script>

<template>
  <section id="demo" class="section clipboard-demo-section">
    <div class="container demo-container">
      <div class="demo-left">
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
                  <div class="empty-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256"><path fill="currentColor" d="M200,32H163.74a47.92,47.92,0,0,0-71.48,0H56A16,16,0,0,0,40,48V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V48A16,16,0,0,0,200,32Zm-72,0a32,32,0,0,1,32,32H96A32,32,0,0,1,128,32Zm72,184H56V48H82.75A47.93,47.93,0,0,0,80,64v8a8,8,0,0,0,8,8h80a8,8,0,0,0,8-8V64a47.93,47.93,0,0,0-2.75-16H200Z"/></svg>
                  </div>
                  <p>{{ searchQuery ? 'No results' : 'Copy something to get started' }}</p>
                </div>
                <div v-else class="clipboard-list" ref="clipboardListRef">
                  <ClipboardItem 
                    v-for="item in displayedItems"
                    :key="item.id"
                    :item="{ ...item, index: item.actualIndex }"
                    :selected="item.actualIndex === selectedIndex"
                    @mouseenter="selectItem(item.actualIndex)"
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
          
          <div class="preview-wrapper">
             <PreviewWindow :item="selectedItem" :preview="selectedPreview" />
          </div>
        </div>
      </div>

      <div class="demo-right">
        <div class="features-list">
          <div v-for="feature in features" :key="feature.title" class="feature-item">
            <i :class="['ph', feature.icon, 'feature-icon']"></i>
            <div>
              <h3 class="feature-title">{{ feature.title }}</h3>
              <p class="feature-description">{{ feature.description }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.clipboard-demo-section {
  background: linear-gradient(to right, var(--accent-dark) 0%, var(--bg-primary) 100%);
  padding: 100px 0;
  color: var(--text-primary);
  height: 100vh;
  display: grid;
  place-items: center;
}

.demo-container {
  display: grid;
  grid-template-columns: 1.2fr 1fr;
  gap: 40px;
  align-items: center;
}

.demo-left {
  position: relative;
}

.demo-wrapper {
  display: flex;
  box-shadow: var(--shadow-2xl);
  border-radius: 12px;
  overflow: hidden;
  height: 500px;
  gap: 20px;
  align-items: flex-start;
  padding: 20px;
}

/* Styles matching ClipboardManager.vue */
.clipboard-demo {
  width: 300px;
  display: flex;
  flex-direction: column;
  font-family: system-ui, sans-serif;
  font-weight: normal;
  gap: 10px;
  padding: 8px;
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  color: var(--text-primary);
  outline: none;
  border-radius: 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12);
}

.search-container {
  margin-top: 3px;
}

.search-input {
  background: var(--bg-input, #fff);
  border: 0.5px solid var(--border-light);
  border-radius: 5px;
  padding: 5px 8px;
  font-family: system-ui;
  box-shadow: var(--shadow-light, 0 1px 2px rgba(0,0,0,0.05));
  color: var(--text-primary);
  width: 100%;
  font-size: 13px;
}

.search-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.7;
}

.search-input:focus {
  outline: none;
  border: none;
  box-shadow: 0 0 0 3px var(--accent-transparent, rgba(37, 99, 235, 0.2));
}

.content-area {
  display: flex;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.items-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow-y: auto;
}

.clipboard-list {
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.empty-state {
  text-align: center;
  color: var(--text-secondary);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.empty-icon {
  font-size: 32px;
  color: var(--text-secondary);
  opacity: 0.7;
  margin-bottom: 8px;
}

.status-bar {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  background: var(--bg-status, #f3f4f6);
  color: var(--text-secondary);
  border-radius: 4px;
  font-size: 0.75em;
  margin-top: auto;
  margin-bottom: 4px;
  flex-shrink: 0;
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

.preview-wrapper {
  width: 300px;
  height: 200px;
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-radius: 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12);
  overflow: hidden;
}

.scroll-indicator {
  text-align: center;
  font-size: 11px;
  color: var(--text-secondary);
  padding: 8px 0 4px;
  opacity: 0.7;
}

.features-list {
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.feature-item {
  display: flex;
  gap: 20px;
  align-items: flex-start;
}

.feature-icon {
  font-size: 24px;
  color: var(--accent-dark);
  padding: 12px;
  background: var(--bg-primary);
  border-radius: 12px;
}

.feature-title {
  font-size: 1.1rem;
  font-weight: 600;
  margin-bottom: 6px;
  color: var(--text-primary);
}

.feature-description {
  font-size: 0.95rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

@media (max-width: 1024px) {
  .demo-container {
    grid-template-columns: 1fr;
    gap: 40px;
  }
  
  .clipboard-demo-section {
    background: linear-gradient(to bottom, var(--accent-transparent), var(--bg-secondary));
  }
  
  .demo-wrapper {
    flex-direction: column;
    height: auto;
    align-items: center;
  }
}
</style>
