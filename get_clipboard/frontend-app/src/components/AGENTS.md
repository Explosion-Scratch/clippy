# Dashboard Components

You are a Vue 3 component specialist working on the dashboard's UI.

## Project Knowledge

- **Tech Stack:** Vue 3, Tailwind CSS
- **Purpose:** Reusable UI components for clipboard browsing

### Component Overview

| Component | Purpose |
|-----------|---------|
| `TopBar.vue` | Header with search, refresh, settings |
| `Sidebar.vue` | Navigation and filters |
| `ItemList.vue` | Scrollable item list |
| `ItemDetail.vue` | Selected item preview |
| `StatsModal.vue` | Usage statistics overlay |
| `SettingsModal.vue` | Preferences modal |
| `ImportModal.vue` | JSON import dialog |
| `ToastContainer.vue` | Notification display |

## Code Style

### Component Pattern
```vue
<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  item: { type: Object, required: true }
})

const emit = defineEmits(['copy', 'delete', 'edit'])

const isExpanded = ref(false)

const preview = computed(() => 
  props.item.summary.slice(0, 100)
)

function handleCopy() {
  emit('copy', props.item.id)
}
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
    <p class="text-gray-900 dark:text-gray-100">{{ preview }}</p>
    <button 
      @click="handleCopy"
      class="mt-2 px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
    >
      Copy
    </button>
  </div>
</template>
```

### List with Virtual Scrolling
```vue
<!-- ItemList.vue pattern -->
<script setup>
const props = defineProps({
  items: Array,
  loading: Boolean
})

const emit = defineEmits(['select'])
</script>

<template>
  <div class="item-list overflow-y-auto">
    <div v-if="loading" class="flex justify-center p-4">
      <span class="animate-spin">⏳</span>
    </div>
    <div 
      v-for="item in items" 
      :key="item.id"
      @click="emit('select', item)"
      class="cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700"
    >
      <ItemRow :item="item" />
    </div>
  </div>
</template>
```

## Conventions

- **Tailwind Classes**: Use utility classes directly in templates
- **Dark Mode**: Support `dark:` variants for all colors
- **Emit Events**: Components emit events, don't manage global state
- **Loading States**: Always show loading indicators

## Boundaries

- ✅ **Always do:**
  - Define props with types
  - Support dark mode
  - Use Tailwind utilities
  - Handle loading/empty states

- ⚠️ **Ask first:**
  - Adding new components
  - Changing component API (props/emits)

- 🚫 **Never do:**
  - Fetch data directly in components (use composables)
  - Use inline styles
  - Skip dark mode support
