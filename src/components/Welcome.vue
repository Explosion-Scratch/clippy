<script setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import ShortcutRecorder from './ShortcutRecorder.vue';

const router = useRouter();

const shortcut = ref('Control+P');
const addToPath = ref(false);
const pathResult = ref('');
const isLoading = ref(false);

async function handleAddToPath() {
  if (!addToPath.value) return;
  
  try {
    const result = await invoke('add_cli_to_path');
    pathResult.value = result;
  } catch (error) {
    pathResult.value = `Error: ${error}`;
  }
}

async function completeSetup() {
  isLoading.value = true;
  
  try {
    if (addToPath.value && !pathResult.value) {
      await handleAddToPath();
    }
    
    const existingSettings = await invoke('get_settings').catch(() => ({}));
    
    await invoke('set_settings', {
      settings: {
        ...existingSettings,
        shortcut: shortcut.value,
        first_run_complete: true,
        cli_in_path: addToPath.value
      }
    });
    
    await invoke('unregister_main_shortcut').catch(() => {});
    await invoke('register_main_shortcut');
    
    router.replace('/');
  } catch (error) {
    console.error('Failed to complete setup:', error);
    isLoading.value = false;
  }
}

function onShortcutChange(newShortcut) {
  shortcut.value = newShortcut;
}
</script>

<template>
  <div class="welcome">
    <div class="welcome-container">
      <div class="header">
        <img src="/icon.png" alt="Clippy" class="app-icon" />
        <h1>Welcome to Clippy</h1>
        <p class="tagline">Your clipboard history, always at hand</p>
      </div>

      <div class="section">
        <h2>Keyboard Shortcut</h2>
        <p class="description">Press this shortcut to show your clipboard history from anywhere.</p>
        
        <ShortcutRecorder 
          v-model="shortcut"
          @change="onShortcutChange"
        />
      </div>

      <div class="section">
        <h2>Command Line Tool</h2>
        <p class="description">Add <code>get_clipboard</code> to your PATH for terminal access.</p>
        
        <label class="checkbox-label">
          <input type="checkbox" v-model="addToPath" />
          <span class="checkmark"></span>
          <span>Install CLI to ~/.local/bin</span>
        </label>
        
        <p v-if="pathResult" class="path-result">{{ pathResult }}</p>
      </div>

      <div class="actions">
        <button 
          class="primary-btn"
          :disabled="isLoading"
          @click="completeSetup"
        >
          {{ isLoading ? 'Setting up...' : 'Get Started' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style lang="less">
.welcome {
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-primary);
  padding: 20px;
  -webkit-user-select: none;
  user-select: none;
}

.welcome-container {
  max-width: 400px;
  width: 100%;
}

.header {
  text-align: center;
  margin-bottom: 24px;
  
  .app-icon {
    width: 64px;
    height: 64px;
    margin-bottom: 12px;
    border-radius: 14px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  }
  
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.5px;
    color: var(--text-primary);
  }
  
  .tagline {
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--text-secondary);
    font-weight: 400;
  }
}

.section {
  background: rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  padding: 14px 16px;
  margin-bottom: 12px;
  
  h2 {
    margin: 0 0 3px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }
  
  .description {
    margin: 0 0 10px;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
    
    code {
      background: rgba(0, 0, 0, 0.06);
      padding: 1px 4px;
      border-radius: 3px;
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 10px;
    }
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-primary);
  
  input[type="checkbox"] {
    appearance: none;
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    border: 1.5px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
    position: relative;
    
    &:checked {
      background: var(--accent);
      border-color: var(--accent);
      
      &::after {
        content: '✓';
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        color: white;
        font-size: 12px;
        font-weight: 600;
      }
    }
    
    &:hover:not(:checked) {
      border-color: rgba(0, 0, 0, 0.3);
    }
  }
}

.path-result {
  margin-top: 10px;
  padding: 10px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 6px;
  font-size: 11px;
  font-family: 'SF Mono', Monaco, monospace;
  color: var(--text-secondary);
  white-space: pre-wrap;
  line-height: 1.5;
}

.actions {
  margin-top: 20px;
}

.primary-btn {
  width: 100%;
  padding: 12px 20px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  
  &:hover:not(:disabled) {
    filter: brightness(1.05);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px var(--accent-transparent, rgba(32, 178, 170, 0.3));
  }
  
  &:active:not(:disabled) {
    transform: translateY(0);
  }
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

@media (prefers-color-scheme: dark) {
  .section {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.1);
  }
  
  .checkbox-label input[type="checkbox"] {
    border-color: rgba(255, 255, 255, 0.3);
    
    &:hover:not(:checked) {
      border-color: rgba(255, 255, 255, 0.5);
    }
  }
  
  .path-result {
    background: rgba(255, 255, 255, 0.06);
  }
  
  .description code {
    background: rgba(255, 255, 255, 0.1);
  }
}
</style>
