<template>
  <div class="fixed inset-0 bg-black/20 z-50 flex items-center justify-center backdrop-blur-sm" @click.self="$emit('close')">
    <div class="bg-white rounded-xl shadow-xl w-[400px] overflow-hidden">
      <div class="p-4 border-b border-gray-100 flex justify-between items-center">
        <h3 class="font-semibold text-sm flex items-center gap-2">
          <PhUploadSimple :size="20" />
          Import Items
        </h3>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <PhX :size="18" />
        </button>
      </div>
      
      <div class="p-6">
        <label class="flex flex-col items-center justify-center w-full h-32 border-2 border-gray-200 border-dashed rounded-lg cursor-pointer hover:bg-gray-50 transition-colors group">
          <div class="flex flex-col items-center justify-center text-gray-400 group-hover:text-gray-600">
            <PhCloudArrowUp :size="32" class="mb-2" />
            <p class="text-xs">Upload JSON file</p>
          </div>
          <input 
            type="file" 
            class="hidden" 
            accept=".json" 
            @change="handleFile"
          />
        </label>
      </div>
    </div>
  </div>
</template>

<script setup>
import { PhUploadSimple, PhCloudArrowUp, PhX } from '@phosphor-icons/vue'

const emit = defineEmits(['close', 'import'])

const handleFile = async (event) => {
  const file = event.target.files[0]
  if (!file) return
  
  try {
    const text = await file.text()
    const data = JSON.parse(text)
    event.target.value = ''
    emit('import', data)
  } catch (error) {
    console.error('Failed to import:', error)
  }
}
</script>

