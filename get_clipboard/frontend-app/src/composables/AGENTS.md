# Dashboard Composables

You are a Vue 3 specialist working on the dashboard's reactive logic.

## Project Knowledge

- **Purpose:** Shared reactive state and API integration
- **Pattern:** Vue 3 Composition API composables

### File Structure

| File | Purpose |
|------|---------|
| `useClipboard.js` | Clipboard items CRUD operations |

## Code Style

### Composable Pattern
```javascript
import { ref, computed, watch } from 'vue'

export function useClipboard() {
  const items = ref([])
  const loading = ref(false)
  const error = ref(null)
  const searchQuery = ref('')
  
  const filteredItems = computed(() => {
    if (!searchQuery.value) return items.value
    const query = searchQuery.value.toLowerCase()
    return items.value.filter(item => 
      item.summary.toLowerCase().includes(query)
    )
  })
  
  async function fetchItems(options = {}) {
    loading.value = true
    error.value = null
    try {
      const params = new URLSearchParams(options)
      const response = await fetch(`/api/items?${params}`)
      if (!response.ok) throw new Error('Failed to fetch')
      items.value = await response.json()
    } catch (e) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }
  
  async function copyItem(id) {
    await fetch(`/api/items/${id}/copy`, { method: 'POST' })
    await fetchItems()
  }
  
  async function deleteItem(id) {
    await fetch(`/api/items/${id}`, { method: 'DELETE' })
    items.value = items.value.filter(i => i.id !== id)
  }
  
  return {
    items,
    loading,
    error,
    searchQuery,
    filteredItems,
    fetchItems,
    copyItem,
    deleteItem,
  }
}
```

### Usage in Components
```vue
<script setup>
import { onMounted } from 'vue'
import { useClipboard } from '../composables/useClipboard'

const { 
  filteredItems, 
  loading, 
  fetchItems, 
  searchQuery 
} = useClipboard()

onMounted(() => fetchItems())
</script>
```

## Conventions

- **Return Object**: Always return an object with named exports
- **Error Handling**: Include error ref and handle fetch errors
- **Loading State**: Track loading state for UI feedback
- **Computed**: Use computed for derived state
- **No Shared State**: Each `use*()` call creates fresh state

## Boundaries

- ✅ **Always do:**
  - Handle loading and error states
  - Use computed for derived data
  - Return consistent interface
  - Use fetch (not axios)

- ⚠️ **Ask first:**
  - Adding new composables
  - Adding global/shared state

- 🚫 **Never do:**
  - Use shared singleton state
  - Skip error handling
  - Return raw refs without computed wrappers
