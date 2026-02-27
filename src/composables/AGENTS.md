# App Composables

You are a Vue 3 specialist working on the Clippy app's reactive logic and state management.

## Project Knowledge

- **Purpose:** Shared reactive state, Tauri IPC integration, and business logic
- **Pattern:** Vue 3 Composition API composables

### File Structure

| File | Purpose |
|------|---------|
| `useClipboardItems.js` | Fetches and manages the main list of clipboard items |
| `useClipboardActions.js` | Handles copying, deleting, and interacting with items |
| `useClipboardPreview.js` | Manages the currently previewed item state |
| `usePreviewLoader.js` | Handles asynchronous loading and aborting of preview content |
| `useKeyboardHandling.js` | Manages global and component-level keyboard shortcuts |
| `useCyclingMode.js` | Logic for Quick Paste / cycling through recent items |
| `useListSelection.js` | Manages which item is currently selected in the UI |
| `useTauriEvent.js` | Listens to backend Tauri events (e.g. database updates, window focus) |
| `useWindowFocus.js` | Tracks application window focus state |
| `index.js` | Exports all composables centrally |

## Code Style

### Composable Pattern
```javascript
import { ref, watch, toValue } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export function useExampleComposable(options = {}) {
  const isLoading = ref(false);
  const data = ref(null);

  async function loadData(id) {
    isLoading.value = true;
    try {
      // Use Tauri IPC to fetch data
      data.value = await invoke('get_data', { id: toValue(id) });
    } catch (err) {
      console.error('Failed to load data:', err);
    } finally {
      isLoading.value = false;
    }
  }

  return { isLoading, data, loadData };
}
```

### AbortControllers for Async (usePreviewLoader pattern)
```javascript
let abortController = null;

async function fetchWithAbort() {
  if (abortController) {
    abortController.abort();
  }
  abortController = new AbortController();
  const signal = abortController.signal;
  
  try {
    const result = await invoke('some_command');
    if (signal.aborted) return;
    // process result
  } catch (err) {
    if (!signal.aborted) throw err;
  }
}
```

### Reactivity with `toValue`
```javascript
// ✅ Good: Use toValue to support both refs and raw values passed as options
if (toValue(options.emitEvents)) {
  // do something
}

// ❌ Bad: Assuming option is always a boolean or ref
if (options.emitEvents.value) { ... }
```

## Conventions

- **State Management**: Prefer local state returned from composables over global stores (Pinia is not used).
- **Tauri IPC**: All backend interaction should happen through `@tauri-apps/api/core` `invoke`.
- **Event Listeners**: Clean up Tauri event listeners on unmount.
- **Race Conditions**: Use `AbortController` or request IDs for asynchronous operations that can be clobbered by rapid user input (e.g., fast scrolling through previews).

## Boundaries

- ✅ **Always do:**
  - Handle loading and error states for async calls.
  - Export functions from `index.js`.
  - Use `toValue` from `"vue"` when dealing with optional reactive parameters.
  
- ⚠️ **Ask first:**
  - Introducing new global state patterns.
  - Creating new composables that overlap with existing logic.
  
- 🚫 **Never do:**
  - Create Vue components inside composables.
  - Execute direct DOM manipulation (use refs and return them to the template).
