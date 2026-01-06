# Main Vue Frontend

You are a Vue 3 specialist working on the main Clippy app interface.

## Project Knowledge

- **Tech Stack:** Vue 3.4+ (Composition API), LESS, Tauri IPC
- **Purpose:** Clipboard browsing, preview, and settings UI
- **Entry:** `main.js` → `App.vue` → Router → Views

### File Structure

| File | Purpose |
|------|---------|
| `App.vue` | Root component, global styles, theme setup |
| `router.js` | View routing (Manager, Settings, Preview) |
| `main.js` | Vue app bootstrap |
| `components/` | UI components |
| `composables/` | Reactive logic (clipboard actions, items, preview, keyboard, etc.) |
| `utils/` | Shared utilities |

## Commands

```bash
bun run tauri dev    # Development with hot reload
bun run build        # Production build (via Tauri)
```

## Code Style

### Component Structure
```vue
<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const items = ref([])
const selected = ref(null)

const filteredItems = computed(() =>
  items.value.filter(i => i.visible)
)

onMounted(async () => {
  items.value = await invoke('get_items')
})
</script>

<template>
  <div class="container">
    <ClipboardItem v-for="item in filteredItems" :key="item.id" :item="item" />
  </div>
</template>

<style scoped lang="less">
.container {
  background: transparent;
  color: var(--text-primary);
}
</style>
```

### Tauri IPC Pattern
```javascript
// ✅ Good - proper error handling
const result = await invoke('get_preview_content', { id })
  .catch(err => console.error('Preview failed:', err))

// ❌ Bad - no error handling
const result = await invoke('get_preview_content', { id })
```

## Conventions

- **Composition API**: Always use `<script setup>`, never Options API
- **Styling**: LESS with CSS variables (`--accent`, `--text-primary`, `--bg-primary`)
- **Transparency**: Maintain `background: transparent` on root for macOS vibrancy
- **IPC**: Use `@tauri-apps/api/core` `invoke` for all backend operations
- **Events**: Use `@tauri-apps/api/event` `listen` for backend-to-frontend events

## Boundaries

- ✅ **Always do:**
  - Use `<script setup>` Composition API
  - Scope styles or use CSS variables from `App.vue`
  - Handle IPC errors gracefully

- ⚠️ **Ask first:**
  - Adding new routes
  - Modifying global styles in `App.vue`
  - Adding new IPC commands

- 🚫 **Never do:**
  - Access filesystem directly (use Tauri commands)
  - Use hardcoded hex colors (use theme variables)
  - Use Options API or mixins
  - Add global CSS that could pollute other components
