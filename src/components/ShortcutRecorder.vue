<script setup>
import { ref, onMounted, onUnmounted, defineProps, defineEmits, watch } from 'vue';

const props = defineProps({
  modelValue: {
    type: String,
    default: 'Control+P'
  },
  showLabel: {
    type: Boolean,
    default: false
  },
  compact: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['update:modelValue', 'change', 'recording']);

const displayShortcut = ref('');
const isRecording = ref(false);

const modifierMap = {
  Control: '⌃',
  Alt: '⌥',
  Shift: '⇧',
  Meta: '⌘'
};

const keyDisplayMap = {
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
  Escape: 'Esc',
  Backspace: '⌫',
  Delete: '⌦',
  Enter: '↵',
  Tab: '⇥',
  Space: 'Space'
};

const codeToKeyMap = {
  KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E',
  KeyF: 'F', KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J',
  KeyK: 'K', KeyL: 'L', KeyM: 'M', KeyN: 'N', KeyO: 'O',
  KeyP: 'P', KeyQ: 'Q', KeyR: 'R', KeyS: 'S', KeyT: 'T',
  KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X', KeyY: 'Y',
  KeyZ: 'Z',
  Digit0: '0', Digit1: '1', Digit2: '2', Digit3: '3', Digit4: '4',
  Digit5: '5', Digit6: '6', Digit7: '7', Digit8: '8', Digit9: '9',
  F1: 'F1', F2: 'F2', F3: 'F3', F4: 'F4', F5: 'F5', F6: 'F6',
  F7: 'F7', F8: 'F8', F9: 'F9', F10: 'F10', F11: 'F11', F12: 'F12',
  Escape: 'Escape', Backspace: 'Backspace', Tab: 'Tab',
  Enter: 'Enter', Space: 'Space',
  ArrowUp: 'ArrowUp', ArrowDown: 'ArrowDown',
  ArrowLeft: 'ArrowLeft', ArrowRight: 'ArrowRight',
  Delete: 'Delete', Home: 'Home', End: 'End',
  PageUp: 'PageUp', PageDown: 'PageDown'
};

function formatShortcutDisplay(shortcutStr) {
  const parts = shortcutStr.split('+');
  return parts.map(part => {
    if (modifierMap[part]) return modifierMap[part];
    if (keyDisplayMap[part]) return keyDisplayMap[part];
    return part.length === 1 ? part.toUpperCase() : part;
  }).join('');
}

function formatShortcutInternal(e) {
  const parts = [];
  if (e.ctrlKey) parts.push('Control');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Meta');
  
  const keyFromCode = codeToKeyMap[e.code];
  if (keyFromCode && !['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'].includes(e.code)) {
    parts.push(keyFromCode);
  }
  
  return parts.join('+');
}

function handleKeyDown(e) {
  if (!isRecording.value) return;
  
  e.preventDefault();
  e.stopPropagation();
  
  if (['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'].includes(e.code)) {
    return;
  }
  
  const formatted = formatShortcutInternal(e);
  if (formatted.includes('+')) {
    displayShortcut.value = formatShortcutDisplay(formatted);
    isRecording.value = false;
    emit('recording', false);
    document.removeEventListener('keydown', handleKeyDown);
    emit('update:modelValue', formatted);
    emit('change', formatted);
  }
}

function startRecording() {
  isRecording.value = true;
  emit('recording', true);
  document.addEventListener('keydown', handleKeyDown);
}

function cancelRecording() {
  isRecording.value = false;
  emit('recording', false);
  document.removeEventListener('keydown', handleKeyDown);
}

watch(() => props.modelValue, (newVal) => {
  displayShortcut.value = formatShortcutDisplay(newVal);
}, { immediate: true });

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown);
});
</script>

<template>
  <div class="shortcut-recorder" :class="{ compact }">
    <label v-if="showLabel" class="label">Keyboard Shortcut</label>
    
    <div class="shortcut-row">
      <div 
        class="shortcut-display" 
        :class="{ recording: isRecording }"
        @click="startRecording"
      >
        <span v-if="!isRecording" class="shortcut-keys">{{ displayShortcut }}</span>
        <span v-else class="recording-hint">Press a key combination...</span>
      </div>
      
      <button 
        v-if="isRecording" 
        class="btn cancel"
        @click="cancelRecording"
      >
        Cancel
      </button>
      <button 
        v-else 
        class="btn change"
        @click="startRecording"
      >
        Change
      </button>
    </div>
  </div>
</template>

<style lang="less" scoped>
.shortcut-recorder {
  .label {
    display: block;
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  
  .shortcut-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .shortcut-display {
    flex: 1;
    background: rgba(0, 0, 0, 0.04);
    border: 1px solid var(--settings-border-color, rgba(0, 0, 0, 0.1));
    border-radius: 6px;
    padding: 8px 14px;
    text-align: center;
    cursor: pointer;
    transition: all 0.2s;
    
    &:hover {
      background: rgba(0, 0, 0, 0.06);
    }
    
    &.recording {
      background: var(--accent-transparent, rgba(32, 178, 170, 0.15));
      border-color: var(--accent);
      box-shadow: 0 0 0 2px var(--accent-transparent, rgba(32, 178, 170, 0.2));
    }
    
    .shortcut-keys {
      font-size: 18px;
      font-weight: 500;
      letter-spacing: 2px;
      color: var(--text-primary);
    }
    
    .recording-hint {
      font-size: 12px;
      color: var(--accent);
    }
  }
  
  .btn {
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    border: 1px solid rgba(0, 0, 0, 0.1);
    
    &.change {
      background: rgba(0, 0, 0, 0.04);
      color: var(--text-primary);
      
      &:hover {
        background: rgba(0, 0, 0, 0.08);
      }
    }
    
    &.cancel {
      background: rgba(255, 59, 48, 0.1);
      color: #FF3B30;
      border-color: rgba(255, 59, 48, 0.2);
      
      &:hover {
        background: rgba(255, 59, 48, 0.15);
      }
    }
  }
  
  &.compact {
    .shortcut-display {
      padding: 6px 12px;
      border-radius: 4px;
      
      .shortcut-keys {
        font-size: 14px;
        letter-spacing: 1px;
      }
      
      .recording-hint {
        font-size: 10px;
      }
    }
    
    .btn {
      padding: 6px 12px;
      font-size: 11px;
    }
  }
}

@media (prefers-color-scheme: dark) {
  .shortcut-recorder {
    .shortcut-display {
      background: rgba(255, 255, 255, 0.06);
      border-color: rgba(255, 255, 255, 0.15);
      
      &:hover {
        background: rgba(255, 255, 255, 0.1);
      }
    }
    
    .btn.change {
      background: rgba(255, 255, 255, 0.08);
      border-color: rgba(255, 255, 255, 0.15);
      
      &:hover {
        background: rgba(255, 255, 255, 0.12);
      }
    }
  }
}
</style>
