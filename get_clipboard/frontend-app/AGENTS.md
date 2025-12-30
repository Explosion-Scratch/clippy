# Dashboard Frontend App

You are a Vue 3 specialist working on get_clipboard's web dashboard.

## Project Knowledge

- **Tech Stack:** Vue 3 (Composition API), Vite, Tailwind CSS
- **Purpose:** Web-based clipboard history browser served by the API
- **Entry:** `index.html` → `src/main.js` → `src/App.vue`

### Directory Structure

| Path | Purpose |
|------|---------|
| `src/App.vue` | Main app component |
| `src/main.js` | Vue app bootstrap |
| `src/style.css` | Global styles |
| `src/components/` | UI components |
| `src/composables/` | Vue composables |
| `src/assets/` | Static assets |

## Commands

```bash
cd get_clipboard/frontend-app
bun install                    # Install dependencies
bun run dev                    # Development server
bun run build                  # Production build to frontend-dist/
```

## Code Style

### Component Pattern
```vue
<script setup>
import { ref, onMounted } from 'vue'
import { useClipboard } from './composables/useClipboard'

const { items, loading, refresh } = useClipboard()

onMounted(() => {
  refresh()
})
</script>

<template>
  <div class="dashboard">
    <TopBar @refresh="refresh" />
    <ItemList :items="items" :loading="loading" />
    <ItemDetail v-if="selected" :item="selected" />
  </div>
</template>

<style scoped>
/* Tailwind classes preferred, scoped for overrides */
</style>
```

### API Integration
```javascript
// composables/useClipboard.js
export function useClipboard() {
  const items = ref([])
  const loading = ref(false)
  
  async function refresh() {
    loading.value = true
    try {
      const response = await fetch('/api/items')
      items.value = await response.json()
    } finally {
      loading.value = false
    }
  }
  
  return { items, loading, refresh }
}
```

## Conventions

- **Tailwind CSS**: Use utility classes, component CSS for complex overrides
- **Composables**: Extract shared reactive logic to `composables/`
- **API Prefix**: All API calls go to `/api/` (proxied in dev, served in prod)
- **Relative URLs**: Dashboard is served from API, use relative paths

## Boundaries

- ✅ **Always do:**
  - Use `<script setup>` Composition API
  - Use Tailwind utility classes
  - Handle loading and error states
  - Use fetch for API calls (not axios)

- ⚠️ **Ask first:**
  - Adding new dependencies
  - Changing build configuration
  - Modifying API integration

- 🚫 **Never do:**
  - Use Options API
  - Access clipboard directly (use API)
  - Add external CDN resources
