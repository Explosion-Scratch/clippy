import { ref, reactive, computed, onMounted, onUnmounted, toValue } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit as tauriEmit } from "@tauri-apps/api/event";
import { getFilteredShortcuts } from "../utils/itemShortcuts";

const CODE_TO_KEY_MAP = {
    KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E',
    KeyF: 'F', KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J',
    KeyK: 'K', KeyL: 'L', KeyM: 'M', KeyN: 'N', KeyO: 'O',
    KeyP: 'P', KeyQ: 'Q', KeyR: 'R', KeyS: 'S', KeyT: 'T',
    KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X', KeyY: 'Y', KeyZ: 'Z',
    Digit0: '0', Digit1: '1', Digit2: '2', Digit3: '3', Digit4: '4',
    Digit5: '5', Digit6: '6', Digit7: '7', Digit8: '8', Digit9: '9',
    F1: 'F1', F2: 'F2', F3: 'F3', F4: 'F4', F5: 'F5', F6: 'F6',
    F7: 'F7', F8: 'F8', F9: 'F9', F10: 'F10', F11: 'F11', F12: 'F12',
    Period: '.', Comma: ',', Slash: '/', Backslash: '\\',
    BracketLeft: '[', BracketRight: ']',
    Semicolon: ';', Quote: "'", Backquote: '`',
    Minus: '-', Equal: '='
};

/**
 * Parses a shortcut string like "Control+Shift+P" into a structured object.
 * @param {string} shortcutStr - The shortcut string to parse
 * @returns {Object} Parsed shortcut with ctrl, alt, shift, meta, and code properties
 */
export function parseShortcutString(shortcutStr) {
    const parts = shortcutStr.split('+').map(p => p.trim().toLowerCase());
    const result = { ctrl: false, alt: false, shift: false, meta: false, code: '' };
    
    for (const part of parts) {
        if (['control', 'ctrl'].includes(part)) result.ctrl = true;
        else if (['alt', 'option'].includes(part)) result.alt = true;
        else if (part === 'shift') result.shift = true;
        else if (['super', 'meta', 'cmd', 'command'].includes(part)) result.meta = true;
        else {
            for (const [code, key] of Object.entries(CODE_TO_KEY_MAP)) {
                if (key.toLowerCase() === part || code.toLowerCase() === part) {
                    result.code = code;
                    break;
                }
            }
        }
    }
    return result;
}

/**
 * Composable for keyboard event handling, shortcut detection, and state management.
 * Self-contained - sets up and cleans up event listeners automatically.
 * 
 * @param {Object} options - Configuration options
 * @param {boolean} [options.autoRegister=true] - Whether to auto-register keyboard listeners
 * @param {Function} [options.onKeyDown] - Callback for keydown events
 * @param {Function} [options.onKeyUp] - Callback for keyup events
 * @param {boolean|Ref<boolean>} [options.emitToPreview=true] - Whether to emit state to preview window
 * @returns {Object} Keyboard state and methods
 */
export function useKeyboardHandling(options = {}) {
    const { 
        autoRegister = true,
        onKeyDown = null,
        onKeyUp = null,
        emitToPreview = true
    } = options;
    
    const configuredShortcut = ref({ ctrl: true, alt: false, shift: false, meta: false, code: 'KeyP' });
    const isModifierPressed = ref(false);
    const isInitialized = ref(false);
    
    const state = reactive({
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        currentlyPressed: [],
        itemShortcuts: []
    });

    let keydownHandler = null;
    let keyupHandler = null;

    async function loadConfiguredShortcut() {
        try {
            const shortcutStr = await invoke("get_configured_shortcut");
            configuredShortcut.value = parseShortcutString(shortcutStr);
            return configuredShortcut.value;
        } catch (e) {
            console.error("Failed to load configured shortcut:", e);
            return configuredShortcut.value;
        }
    }

    function matchesConfiguredShortcut(event) {
        const s = configuredShortcut.value;
        return event.ctrlKey === s.ctrl &&
               event.altKey === s.alt &&
               event.shiftKey === s.shift &&
               event.metaKey === s.meta &&
               event.code === s.code;
    }

    function hasAnyShortcutModifier(event) {
        const s = configuredShortcut.value;
        return (s.ctrl && event.ctrlKey) ||
               (s.alt && event.altKey) ||
               (s.shift && event.shiftKey) ||
               (s.meta && event.metaKey);
    }

    function updateState(event) {
        state.ctrl = event.ctrlKey;
        state.alt = event.altKey;
        state.shift = event.shiftKey;
        state.meta = event.metaKey;
        
        const pressed = [];
        if (event.ctrlKey) pressed.push('Control');
        if (event.altKey) pressed.push('Alt');
        if (event.shiftKey) pressed.push('Shift');
        if (event.metaKey) pressed.push('Meta');
        state.currentlyPressed = pressed;
        state.itemShortcuts = getFilteredShortcuts(pressed);
        
        if (toValue(emitToPreview)) {
            tauriEmit("keyboard-state-changed", {
                currentlyPressed: pressed,
                itemShortcuts: state.itemShortcuts,
                ctrl: state.ctrl,
                alt: state.alt,
                shift: state.shift,
                meta: state.meta
            }).catch(err => console.error("Failed to emit keyboard state:", err));
        }
    }

    function resetState() {
        state.ctrl = false;
        state.alt = false;
        state.shift = false;
        state.meta = false;
        state.currentlyPressed = [];
        state.itemShortcuts = [];
        isModifierPressed.value = false;
        
        if (toValue(emitToPreview)) {
            tauriEmit("keyboard-state-changed", {
                currentlyPressed: [],
                itemShortcuts: [],
                ctrl: false,
                alt: false,
                shift: false,
                meta: false
            }).catch(err => console.error("Failed to emit keyboard state:", err));
        }
    }

    function handleKeyDown(event) {
        updateState(event);
        
        if (hasAnyShortcutModifier(event)) {
            isModifierPressed.value = true;
        }
        
        if (onKeyDown) {
            onKeyDown(event, {
                matchesShortcut: matchesConfiguredShortcut(event),
                state
            });
        }
    }

    function handleKeyUp(event) {
        updateState(event);
        
        const s = configuredShortcut.value;
        const modifierReleased = 
            (s.ctrl && event.key === 'Control') ||
            (s.alt && event.key === 'Alt') ||
            (s.shift && event.key === 'Shift') ||
            (s.meta && event.key === 'Meta');
        
        if (modifierReleased) {
            isModifierPressed.value = false;
        }
        
        if (onKeyUp) {
            onKeyUp(event, {
                modifierReleased,
                state
            });
        }
    }

    async function unregisterGlobalShortcut() {
        try {
            await invoke("unregister_main_shortcut");
        } catch (e) {
            console.error("Failed to unregister global shortcut:", e);
        }
    }

    async function registerGlobalShortcut() {
        try {
            await invoke("register_main_shortcut");
        } catch (e) {
            console.error("Failed to register global shortcut:", e);
        }
    }

    function registerListeners() {
        if (keydownHandler) return;
        
        keydownHandler = handleKeyDown;
        keyupHandler = handleKeyUp;
        document.addEventListener("keydown", keydownHandler);
        document.addEventListener("keyup", keyupHandler);
        isInitialized.value = true;
    }

    function unregisterListeners() {
        if (keydownHandler) {
            document.removeEventListener("keydown", keydownHandler);
            keydownHandler = null;
        }
        if (keyupHandler) {
            document.removeEventListener("keyup", keyupHandler);
            keyupHandler = null;
        }
        isInitialized.value = false;
    }

    const currentlyPressed = computed(() => state.currentlyPressed);
    const hasModifiers = computed(() => state.currentlyPressed.length > 0);

    if (autoRegister) {
        onMounted(() => {
            registerListeners();
        });

        onUnmounted(() => {
            unregisterListeners();
        });
    }

    return {
        configuredShortcut,
        isModifierPressed,
        isInitialized,
        state,
        currentlyPressed,
        hasModifiers,
        loadConfiguredShortcut,
        matchesConfiguredShortcut,
        hasAnyShortcutModifier,
        updateState,
        resetState,
        unregisterGlobalShortcut,
        registerGlobalShortcut,
        registerListeners,
        unregisterListeners
    };
}
