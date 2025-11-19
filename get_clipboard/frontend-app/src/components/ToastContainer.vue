<template>
  <div class="fixed bottom-6 right-6 z-[100] flex flex-col gap-2 pointer-events-none">
    <TransitionGroup name="list">
      <div 
        v-for="toast in toasts" 
        :key="toast.id"
        class="bg-white border border-gray-200 shadow-lg rounded-lg px-4 py-3 flex items-center gap-3 pointer-events-auto min-w-[300px]"
      >
        <PhCheckCircle v-if="toast.type === 'success'" :size="20" class="text-green-500" weight="fill" />
        <PhWarningCircle v-else :size="20" class="text-red-500" weight="fill" />
        <div class="flex-1">
          <p class="text-sm font-medium text-gray-900">{{ toast.title }}</p>
          <p v-if="toast.message" class="text-xs text-gray-500">{{ toast.message }}</p>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup>
import { PhCheckCircle, PhWarningCircle } from '@phosphor-icons/vue'

defineProps({
  toasts: Array
})
</script>

<style scoped>
.list-move,
.list-enter-active,
.list-leave-active {
  transition: all 0.3s ease;
}

.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

.list-leave-active {
  position: absolute;
}
</style>

