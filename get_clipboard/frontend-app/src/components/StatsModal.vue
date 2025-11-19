<template>
  <div class="fixed inset-0 bg-black/20 z-50 flex items-center justify-center backdrop-blur-sm" @click.self="$emit('close')">
    <div class="bg-white rounded-xl shadow-2xl w-[800px] max-w-[95vw] max-h-[90vh] overflow-hidden flex flex-col">
      <div class="p-5 border-b border-gray-100 flex justify-between items-center bg-gradient-to-r from-gray-50 to-white">
        <h3 class="font-semibold text-lg flex items-center gap-2">
          <PhChartLineUp :size="24" class="text-blue-600" />
          Library Statistics
        </h3>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600 p-1 rounded-lg hover:bg-gray-100 transition-colors">
          <PhX :size="20" />
        </button>
      </div>
      
      <div class="p-6 overflow-y-auto scrollbar-thin">
        <div v-if="stats" class="space-y-8">
          <div class="grid grid-cols-3 gap-4">
            <div class="stat-card bg-gradient-to-br from-blue-50 to-blue-100/50 border-blue-200">
              <div class="text-2xl font-bold text-blue-600">{{ stats.totalItems }}</div>
              <div class="text-xs text-blue-500 uppercase font-semibold tracking-wider">Total Items</div>
            </div>
            <div class="stat-card bg-gradient-to-br from-green-50 to-green-100/50 border-green-200">
              <div class="text-2xl font-bold text-green-600">{{ formatBytes(stats.totalSize) }}</div>
              <div class="text-xs text-green-500 uppercase font-semibold tracking-wider">Total Size</div>
            </div>
            <div class="stat-card bg-gradient-to-br from-purple-50 to-purple-100/50 border-purple-200">
              <div class="text-2xl font-bold text-purple-600">{{ Object.keys(stats.typeCounts).length }}</div>
              <div class="text-xs text-purple-500 uppercase font-semibold tracking-wider">Item Types</div>
            </div>
          </div>

          <div class="border border-gray-200 rounded-xl p-4 bg-white shadow-sm">
            <h4 class="text-sm font-medium text-gray-700 mb-4 flex items-center gap-2">
              <PhChartBar :size="18" />
              Items Added History
            </h4>
            <div class="h-64">
              <canvas ref="chartCanvas"></canvas>
            </div>
            <div class="mt-4 text-xs text-gray-400 text-center">
              Click on a bar to view items for that day
            </div>
          </div>

          <div v-if="selectedDay" class="border border-gray-200 rounded-xl overflow-hidden flex flex-col max-h-96">
            <div class="bg-gray-50 px-4 py-2 border-b border-gray-200 flex justify-between items-center flex-shrink-0">
              <span class="font-semibold text-sm">{{ selectedDay.date }}</span>
              <button @click="selectedDay = null" class="text-xs text-blue-500 hover:underline">Close</button>
            </div>
            
            <div class="p-2 bg-white overflow-y-auto scrollbar-thin">
              <div v-if="loadingDayItems" class="flex justify-center py-4">
                <div class="spinner w-6 h-6" />
              </div>
              <div v-else class="space-y-1">
                <div 
                  v-for="item in dayItems" 
                  :key="item.id"
                  @click="$emit('load-item', item.id); $emit('close')"
                  class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-50 cursor-pointer border border-transparent hover:border-gray-100 group transition-all"
                >
                  <div class="text-gray-400 group-hover:text-blue-500 flex-shrink-0">
                    <PhTextT v-if="item.type === 'text'" :size="16" />
                    <PhImageIcon v-else-if="item.type === 'image'" :size="16" />
                    <PhFileIcon v-else-if="item.type === 'file'" :size="16" />
                    <PhCube v-else :size="16" />
                  </div>
                  <div class="flex-1 text-xs text-gray-700 truncate font-mono">
                    {{ item.summary || item.data }}
                  </div>
                  <div class="text-[10px] text-gray-400 tabular-nums flex-shrink-0">
                    {{ formatBytes(item.size) }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else class="flex justify-center py-10">
          <div class="spinner w-8 h-8" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick, onMounted } from 'vue'
import { PhChartLineUp, PhChartBar, PhX, PhTextT, PhImage as PhImageIcon, PhFile as PhFileIcon, PhCube } from '@phosphor-icons/vue'
import Chart from 'chart.js/auto'

const props = defineProps({
  stats: Object
})

defineEmits(['close', 'load-item'])

const chartCanvas = ref(null)
const selectedDay = ref(null)
const dayItems = ref([])
const loadingDayItems = ref(false)
let chartInstance = null

const loadDayItems = async () => {
  if (!selectedDay.value) {
    dayItems.value = []
    return
  }
  
  const allIds = []
  for (const type in selectedDay.value.data) {
    allIds.push(...selectedDay.value.data[type].ids)
  }
  
  if (allIds.length === 0) {
    dayItems.value = []
    return
  }

  loadingDayItems.value = true
  try {
    // Basic chunking to avoid URL length limits
    const chunks = []
    let currentChunk = []
    let currentLength = 0
    
    for (const id of allIds) {
      if (currentLength + id.length > 1500) {
        chunks.push(currentChunk)
        currentChunk = []
        currentLength = 0
      }
      currentChunk.push(id)
      currentLength += id.length + 1
    }
    if (currentChunk.length > 0) chunks.push(currentChunk)
    
    const results = []
    for (const chunk of chunks) {
      const res = await fetch(`${window.location.origin}/items?ids=${chunk.join(',')}`)
      if (res.ok) {
        results.push(...await res.json())
      }
    }
    dayItems.value = results
  } catch (e) {
    console.error(e)
  } finally {
    loadingDayItems.value = false
  }
}

watch(selectedDay, () => {
  loadDayItems()
})

const formatBytes = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

const updateChart = () => {
  if (!chartCanvas.value || !props.stats?.history) return
  
  if (chartInstance) chartInstance.destroy()

  const history = props.stats.history
  const dates = Object.keys(history).sort()
  
  const types = ['text', 'image', 'file', 'other']
  const datasets = types.map(type => ({
    label: type.charAt(0).toUpperCase() + type.slice(1),
    data: dates.map(date => history[date]?.[type]?.count || 0),
    backgroundColor: type === 'text' ? '#3b82f6' : type === 'image' ? '#10b981' : type === 'file' ? '#a855f7' : '#f59e0b',
    stack: 'Stack 0',
  }))

  chartInstance = new Chart(chartCanvas.value, {
    type: 'bar',
    data: { labels: dates, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      onClick: (e, elements) => {
        if (elements.length > 0) {
          const index = elements[0].index
          const date = dates[index]
          selectedDay.value = { date, data: history[date] }
        }
      },
      plugins: {
        legend: {
          position: 'bottom',
        }
      }
    }
  })
}

watch(() => props.stats, () => {
  nextTick(updateChart)
}, { immediate: true })

onMounted(() => {
  nextTick(updateChart)
})
</script>

<style scoped>
.stat-card {
  @apply p-4 rounded-xl border transition-all;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
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

