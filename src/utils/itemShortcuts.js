/**
 * Unified clipboard item shortcuts definitions.
 * These are internal shortcuts for item actions (not user-configurable).
 */

export const ITEM_SHORTCUTS = {
  paste: {
    id: 'paste',
    label: 'Paste',
    key: 'Enter',
    modifiers: { metaKey: false, altKey: false, shiftKey: false, ctrlKey: false },
    description: 'Paste the selected item'
  },
  copy: {
    id: 'copy',
    label: 'Copy',
    key: 'Enter',
    modifiers: { metaKey: true, altKey: false, shiftKey: false, ctrlKey: false },
    description: 'Copy the selected item without pasting'
  },
  openDashboard: {
    id: 'openDashboard',
    label: 'Open in dashboard',
    key: 'Enter',
    modifiers: { shiftKey: true, metaKey: false, altKey: false, ctrlKey: false },
    description: 'Open the selected item in the web dashboard'
  }
}

/**
 * Check if a keyboard event matches a shortcut definition.
 * @param {KeyboardEvent} event 
 * @param {Object} shortcut 
 * @returns {boolean}
 */
export function matchesShortcut(event, shortcut) {
  if (event.key !== shortcut.key) return false
  
  const mods = shortcut.modifiers
  return (
    event.metaKey === !!mods.metaKey &&
    event.altKey === !!mods.altKey &&
    event.shiftKey === !!mods.shiftKey &&
    event.ctrlKey === !!mods.ctrlKey
  )
}

/**
 * Find which shortcut (if any) matches the keyboard event.
 * Returns shortcuts in order of specificity (most modifiers first).
 * @param {KeyboardEvent} event 
 * @returns {Object|null} The matching shortcut definition or null
 */
export function findMatchingShortcut(event) {
  const sortedShortcuts = Object.values(ITEM_SHORTCUTS).sort((a, b) => {
    const countMods = (s) => Object.values(s.modifiers).filter(Boolean).length
    return countMods(b) - countMods(a)
  })

  for (const shortcut of sortedShortcuts) {
    if (matchesShortcut(event, shortcut)) {
      return shortcut
    }
  }
  return null
}

/**
 * Handle item shortcuts and execute the appropriate action.
 * @param {KeyboardEvent} event 
 * @param {Object} selectedItem - The currently selected clipboard item
 * @param {Object} actions - Object containing action functions: { paste, copy, copyPlain, pastePlain, openDashboard }
 * @returns {boolean} True if a shortcut was handled, false otherwise
 */
export function handleItemShortcuts(event, selectedItem, actions) {
  if (!selectedItem) return false

  const shortcut = findMatchingShortcut(event)
  if (!shortcut) return false

  event.preventDefault()
  
  const actionFn = actions[shortcut.id]
  if (actionFn) {
    actionFn(selectedItem)
    return true
  }
  
  return false
}

/**
 * Format a shortcut for display (e.g., "⌘+Enter")
 * @param {Object} shortcut 
 * @returns {string}
 */
export function formatShortcut(shortcut) {
  const parts = []
  if (shortcut.modifiers.ctrlKey) parts.push('⌃')
  if (shortcut.modifiers.altKey) parts.push('⌥')
  if (shortcut.modifiers.shiftKey) parts.push('⇧')
  if (shortcut.modifiers.metaKey) parts.push('⌘')
  parts.push(shortcut.key)
  return parts.join('')
}
