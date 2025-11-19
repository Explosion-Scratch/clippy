<template>
  <div class="fixed inset-0 bg-black/20 z-50 flex items-center justify-center backdrop-blur-sm" @click.self="$emit('close')">
    <div class="bg-white rounded-xl shadow-xl w-[500px] overflow-hidden">
      <div class="p-4 border-b border-gray-100 flex justify-between items-center">
        <h3 class="font-semibold text-sm flex items-center gap-2">
          <PhGear :size="20" />
          Settings
        </h3>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <PhX :size="18" />
        </button>
      </div>
      
      <div class="p-6 space-y-6">
        <div>
          <label class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-2">
            Data Directory
          </label>
          <input 
            v-model="tempDir"
            type="text" 
            class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white font-mono focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 outline-none transition-all"
          />
          <div class="flex gap-2 mt-3">
            <button 
              @click="$emit('update', 'move', tempDir)" 
              :disabled="isUnchanged"
              class="flex-1 btn-action bg-gradient-to-r from-blue-600 to-blue-500 text-white hover:from-blue-700 hover:to-blue-600 justify-center disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:transform-none"
            >
              <PhFolderOpen :size="16" />
              Move Data Here
            </button>
            <button 
              @click="$emit('update', 'update', tempDir)" 
              :disabled="isUnchanged"
              class="flex-1 btn-action bg-white border border-gray-200 text-gray-700 hover:bg-gray-50 justify-center disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-white disabled:hover:transform-none"
            >
              <PhPath :size="16" />
              Update Path Only
            </button>
          </div>
          <p class="mt-2 text-[10px] text-gray-400 leading-relaxed">
            "Move Data" transfers existing files to the new location.<br>
            "Update Path" changes the config without moving files.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed } from 'vue'
import { PhGear, PhX, PhFolderOpen, PhPath } from '@phosphor-icons/vue'

const props = defineProps({
  dataDir: String
})

defineEmits(['close', 'update'])

const tempDir = ref(props.dataDir)

const isUnchanged = computed(() => tempDir.value === props.dataDir)

watch(() => props.dataDir, (val) => {
  tempDir.value = val
})
</script>

<style scoped>
.btn-action {
  @apply px-3 py-1.5 rounded-lg text-xs font-medium transition-all flex items-center gap-1.5 shadow-sm;
}

.btn-action:hover {
  transform: translateY(-1px);
}
</style>

