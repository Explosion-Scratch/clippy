# Vue Utilities

You are a JavaScript utility specialist working on shared helpers for Clippy's Vue frontend.

## Project Knowledge

- **Purpose:** Shared utility functions for UI and keyboard handling
- **Consumers:** Vue components in `../components/`

### File Overview

| File | Purpose |
|------|---------|
| `itemShortcuts.js` | Keyboard shortcut generation and matching |
| `ui.js` | DOM manipulation and UI helpers |

## Code Style

### Module Pattern
```javascript
// ✅ Good - pure functions, well-documented
/**
 * Generate a keyboard shortcut for a given index
 * @param {number} index - Item index (0-based)
 * @returns {string} Shortcut key combo
 */
export function getShortcut(index) {
  if (index < 10) return `⌘${index}`
  return null
}

// ❌ Bad - side effects, no documentation
export function getShortcut(index) {
  console.log('getting shortcut')
  return index < 10 ? `⌘${index}` : null
}
```

### Keyboard Utilities (`itemShortcuts.js`)
```javascript
// Shortcut matching pattern
export function matchesShortcut(event, shortcut) {
  const key = shortcut.slice(-1)
  const meta = shortcut.includes('⌘')
  return event.key === key && event.metaKey === meta
}

// Index-based shortcuts (⌘1-9, ⌘0)
export function getIndexShortcut(index) {
  if (index >= 0 && index <= 9) {
    return `⌘${index === 9 ? 0 : index + 1}`
  }
  return null
}
```

### DOM Utilities (`ui.js`)
```javascript
// Scroll item into view
export function scrollIntoViewIfNeeded(element, container) {
  const rect = element.getBoundingClientRect()
  const containerRect = container.getBoundingClientRect()

  if (rect.top < containerRect.top || rect.bottom > containerRect.bottom) {
    element.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }
}
```

## Conventions

- **Pure Functions**: Utilities should be pure with no side effects
- **JSDoc**: Document all exported functions
- **No Vue Imports**: Keep utilities framework-agnostic
- **No DOM Globals**: Pass elements as arguments, don't query directly

## Boundaries

- ✅ **Always do:**
  - Write pure functions
  - Document with JSDoc
  - Export from module (no default exports)
  - Write unit-testable code

- ⚠️ **Ask first:**
  - Adding new utility files
  - Adding Vue-specific helpers (may belong in composables)

- 🚫 **Never do:**
  - Import Vue or Tauri in utilities
  - Mutate function arguments
  - Use global DOM queries (`document.querySelector`)
  - Add side effects (logging, network calls)
