<script setup>
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps({
  modelValue: {
    type: String,
    default: '#20b2aa'
  },
  compact: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['update:modelValue', 'change']);

const ACCENT_COLORS = [
  { name: 'Teal', hex: '#20b2aa' },
  { name: 'Blue', hex: '#3b82f6' },
  { name: 'Purple', hex: '#8b5cf6' },
  { name: 'Pink', hex: '#ec4899' },
  { name: 'Orange', hex: '#f97316' },
  { name: 'Green', hex: '#22c55e' },
  { name: 'Red', hex: '#ef4444' },
];

const isCustomColor = computed(() => !ACCENT_COLORS.some(c => c.hex === props.modelValue));

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function applyAccentColor(hex) {
  document.documentElement.style.setProperty('--accent', hex);
  document.documentElement.style.setProperty('--accent-transparent', hexToRgba(hex, 0.25));
}

async function selectColor(hex) {
  applyAccentColor(hex);
  emit('update:modelValue', hex);
  emit('change', hex);
  
  try {
    const settings = await invoke('get_settings');
    await invoke('set_settings', {
      settings: {
        ...settings,
        accent_color: hex
      }
    });
  } catch (error) {
    console.error('Failed to save accent color:', error);
  }
}

function onCustomColorInput(event) {
  selectColor(event.target.value);
}

onMounted(() => {
  if (props.modelValue) {
    applyAccentColor(props.modelValue);
  }
});
</script>

<template>
  <div class="color-picker" :class="{ compact }">
    <button
      v-for="color in ACCENT_COLORS"
      :key="color.hex"
      class="color-swatch"
      :class="{ selected: modelValue === color.hex }"
      :style="{ backgroundColor: color.hex }"
      :title="color.name"
      @click="selectColor(color.hex)"
    >
      <span v-if="modelValue === color.hex" class="checkmark">✓</span>
    </button>
    
    <label class="color-swatch custom" :class="{ selected: isCustomColor }" title="Custom color">
      <input 
        type="color" 
        :value="modelValue" 
        @input="onCustomColorInput"
        class="color-input"
      />
      <svg v-if="!isCustomColor" class="eyedropper-icon" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="m2 22 1-1h3l9-9"/>
        <path d="M3 21v-3l9-9"/>
        <path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>
      </svg>
      <span v-if="isCustomColor" class="checkmark">✓</span>
    </label>
  </div>
</template>

<style lang="less">
.color-picker {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  
  &.compact {
    gap: 6px;
    
    .color-swatch {
      width: 24px;
      height: 24px;
      
      .checkmark {
        font-size: 10px;
      }
      
      .eyedropper-icon {
        width: 10px;
        height: 10px;
      }
    }
  }
  
  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    
    &:hover {
      transform: scale(1.1);
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
    }
    
    &.selected {
      border-color: var(--text-primary);
      box-shadow: 0 0 0 2px var(--settings-bg-primary, var(--bg-primary)), 0 0 0 4px var(--text-primary);
    }
    
    .checkmark {
      color: white;
      font-size: 12px;
      font-weight: bold;
      text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    }
    
    &.custom {
      background: #3d3d3d;
      position: relative;
      cursor: pointer;
      
      .color-input {
        position: absolute;
        inset: 0;
        opacity: 0;
        cursor: pointer;
        width: 100%;
        height: 100%;
      }
      
      .eyedropper-icon {
        color: white;
        opacity: 0.9;
      }
    }
  }
}

@media (prefers-color-scheme: light) {
  .color-picker .color-swatch.custom {
    background: #c0c0c0;
    
    .eyedropper-icon {
      color: #333;
    }
  }
}
</style>
