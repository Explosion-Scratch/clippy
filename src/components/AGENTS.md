# Vue Components

You are a Vue 3 component specialist working on Clippy's UI components.

## Project Knowledge

- **Tech Stack:** Vue 3.4+ (Composition API), LESS
- **Pattern:** Single-file components with scoped styles

### Component Overview

| Component | Purpose | Size |
|-----------|---------|------|
| `ClipboardManager.vue` | Main list view, search, keyboard navigation | Large (39KB) |
| `ClipboardItem.vue` | Individual item row with preview | Medium |
| `PreviewPane.vue` | Inline preview panel | Medium |
| `PreviewWindow.vue` | Standalone preview window | Small |
| `Settings.vue` | App settings form | Large |
| `ShortcutRecorder.vue` | Keyboard shortcut input | Medium |
| `AccentColorPicker.vue` | Theme color selector | Medium |
| `Welcome.vue` | Onboarding wizard | Medium |

## Code Style

### Props & Emits
```vue
<script setup>
const props = defineProps({
  item: { type: Object, required: true },
  selected: { type: Boolean, default: false }
})

const emit = defineEmits(['select', 'preview', 'copy'])

function handleClick() {
  emit('select', props.item.id)
}
</script>
```

### Keyboard Handling
```javascript
// Pattern from ClipboardManager.vue
function handleKeydown(e) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectNext()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    copySelected()
  }
}
```

### Preview Pattern
```vue
<!-- Inline preview (PreviewPane) vs floating (PreviewWindow) -->
<PreviewPane v-if="showInlinePreview" :item="selectedItem" />

<!-- Floating preview managed by Tauri backend -->
<script setup>
import { invoke } from '@tauri-apps/api/core'

async function showFloatingPreview(id) {
  await invoke('preview_item', { id })
}
</script>
```

## Conventions

- **Large Components**: `ClipboardManager.vue` and `Settings.vue` are intentionally large; keep logic there rather than over-fragmenting
- **Preview Architecture**: `PreviewPane` is inline, `PreviewWindow` is a separate Tauri window
- **Keyboard First**: All major actions must have keyboard shortcuts
- **Theming**: Use LESS variables that reference CSS custom properties
- **Event-Driven Sync**: Use `listen` from `@tauri-apps/api/event` to keep UI in sync with backend state rather than polling.
- **Focus Management**: Reset UI state (search, selection) when the window gains focus to provide a fresh starting point.

## Boundaries

- ✅ **Always do:**
  - Define props with types and defaults
  - Use `defineEmits` for all events
  - Keep styles scoped with `lang="less"`
  - Support keyboard navigation
  - Use Tauri event listeners for real-time data updates

- ⚠️ **Ask first:**
  - Splitting large components (they may be intentionally monolithic)
  - Adding new components
  - Changing keyboard shortcuts

- 🚫 **Never do:**
  - Use Options API
  - Add unscoped global styles
  - Hardcode colors (use variables)
  - Skip keyboard accessibility
