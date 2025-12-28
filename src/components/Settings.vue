<script setup>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { save, open as openDialog, ask } from '@tauri-apps/plugin-dialog';
import { writeFile, readFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { openPath } from '@tauri-apps/plugin-opener';
import ShortcutRecorder from './ShortcutRecorder.vue';
import AccentColorPicker from './AccentColorPicker.vue';

const appVersion = ref('0.1.0');
const itemCount = ref(0);
const databaseSize = ref(0);
const currentDataDir = ref('');
const isDeleting = ref(false);
const isExporting = ref(false);
const isImporting = ref(false);

const shortcut = ref('Control+P');
const displayShortcut = ref('⌃P');
const isSavingShortcut = ref(false);
const isRecordingShortcut = ref(false);

const accentColor = ref('#20b2aa');

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

function formatShortcutDisplay(shortcutStr) {
  const parts = shortcutStr.split('+');
  return parts.map(part => {
    if (modifierMap[part]) return modifierMap[part];
    if (keyDisplayMap[part]) return keyDisplayMap[part];
    return part.length === 1 ? part.toUpperCase() : part;
  }).join('');
}

async function onShortcutChange(newShortcut) {
  try {
    isSavingShortcut.value = true;
    
    await invoke('unregister_main_shortcut').catch(() => {});
    
    const settings = await invoke('get_settings');
    await invoke('set_settings', {
      settings: {
        ...settings,
        shortcut: newShortcut
      }
    });
    
    await invoke('register_main_shortcut');
    
    shortcut.value = newShortcut;
    displayShortcut.value = formatShortcutDisplay(newShortcut);
  } catch (error) {
    console.error('Failed to save shortcut:', error);
    alert('Failed to save shortcut: ' + error);
  } finally {
    isSavingShortcut.value = false;
  }
}

async function loadShortcut() {
  try {
    const stored = await invoke('get_configured_shortcut');
    shortcut.value = stored;
    displayShortcut.value = formatShortcutDisplay(stored);
  } catch (error) {
    console.error('Failed to load shortcut:', error);
  }
}

async function loadStats() {
  try {
    itemCount.value = await invoke('db_get_count');
    databaseSize.value = await invoke('db_get_size');
    currentDataDir.value = await invoke('get_sidecar_dir');
  } catch (error) {
    console.error('Failed to get stats:', error);
  }
}

async function loadAccentColor() {
  try {
    const settings = await invoke('get_settings');
    accentColor.value = settings.accent_color || '#20b2aa';
  } catch (error) {
    console.error('Failed to load accent color:', error);
  }
}

async function reloadAllSettings() {
  await loadShortcut();
  await loadStats();
  await loadAccentColor();
}

function onAccentColorChange(hex) {
  accentColor.value = hex;
}

async function openDataDirectory() {
  if (currentDataDir.value) {
    try {
      await openPath(currentDataDir.value);
    } catch (error) {
      console.error('Failed to open directory:', error);
    }
  }
}

async function changeDataDirectory() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: 'Select Data Directory',
    });

    if (selected) {
      const shouldMove = await ask(
        `Do you want to move your existing data to "${selected}"?\n\nSelect 'Yes' to move existing data.\nSelect 'No' to start fresh or use existing data in that folder.`,
        {
          title: 'Move Data?',
          kind: 'info',
          okLabel: 'Yes, Move Data',
          cancelLabel: 'No, Start Fresh'
        }
      );

      const mode = shouldMove ? 'move' : 'update';
      await invoke('set_sidecar_dir', { mode, path: selected });
      
      await loadStats();
    }
  } catch (error) {
    console.error('Failed to change directory:', error);
    alert('Failed to change directory: ' + error);
  }
}

async function exportDatabase() {
  try {
    isExporting.value = true;
    
    const filePath = await save({
      title: 'Export Clipboard Database',
      filters: [
        {
          name: 'JSON Files',
          extensions: ['json']
        }
      ]
    });

    if (filePath) {
      const jsonData = await invoke('db_export_all');
      await writeFile(filePath, new TextEncoder().encode(jsonData));
      alert('Database exported successfully!');
    }
  } catch (error) {
    console.error('Failed to export database:', error);
    alert('Failed to export database: ' + error);
  } finally {
    isExporting.value = false;
  }
}

async function importDatabase() {
  try {
    isImporting.value = true;
    
    const filePath = await openDialog({
      title: 'Import Clipboard Database',
      filters: [
        {
          name: 'JSON Files',
          extensions: ['json']
        }
      ]
    });

    if (filePath) {
      const jsonDataBytes = await readFile(filePath);
      const jsonData = new TextDecoder().decode(jsonDataBytes);
      const result = await invoke('db_import_all', { jsonData });
      alert(result);
      await loadStats();
    }
  } catch (error) {
    console.error('Failed to import database:', error);
    alert('Failed to import database: ' + error);
  } finally {
    isImporting.value = false;
  }
}

async function deleteAllData() {
  try {
    const confirmed = await ask('Are you sure you want to delete all clipboard items? This action cannot be undone.', {
      title: 'Confirm Delete All Data',
      kind: 'warning'
    });
    
    if (confirmed) {
      isDeleting.value = true;
      const result = await invoke('db_delete_all');
      alert(result);
      await loadStats();
    }
  } catch (error) {
    console.error('Failed to delete all data:', error);
    alert('Failed to delete data: ' + error);
  } finally {
    isDeleting.value = false;
  }
}

async function closeSettings() {
  const window = getCurrentWindow();
  await window.close();
}

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

let unlistenFocus = null;

onMounted(async () => {
  appVersion.value = await getVersion();
  await reloadAllSettings();
  
  const currentWindow = getCurrentWindow();
  unlistenFocus = await currentWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      reloadAllSettings();
    }
  });
  
  document.addEventListener('keyup', async (e) => {
    if (e.key === 'Escape') {
      if (isRecordingShortcut.value) return;
      await closeSettings();
    }
  });
});

onUnmounted(() => {
  if (unlistenFocus) {
    unlistenFocus();
  }
});
</script>

<template>
  <div class="settings">
    <div class="settings-header">
      <div class="app-info">
        <img src="/icon.png" alt="App Icon" class="app-icon" />
        <div class="app-details">
          <h1>Clippy Settings</h1>
          <p class="version">Version {{ appVersion }}</p>
        </div>
      </div>
      <div class="header-actions">
        <a href="https://github.com/user/clippy" target="_blank" class="icon-button" title="View on GitHub">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
        </a>
        <button @click="closeSettings" class="close-button">×</button>
      </div>
    </div>

    <div class="settings-content">
      <div class="section">
        <h2>Storage</h2>
        <div class="path-container">
          <p class="path-label">Current Data Directory:</p>
          <a 
            href="#" 
            @click.prevent="openDataDirectory" 
            class="path-value clickable" 
            title="Open in Finder"
          >
            {{ currentDataDir || 'Loading...' }}
          </a>
        </div>
        <button @click="changeDataDirectory" class="action-button">Change Directory...</button>
      </div>

      <div class="section">
        <h2>Keyboard Shortcut</h2>
        <p class="section-description">Press this shortcut to show the clipboard manager from anywhere.</p>
        
        <ShortcutRecorder 
          v-model="shortcut"
          compact
          @change="onShortcutChange"
          @recording="(val) => isRecordingShortcut = val"
        />
      </div>

      <div class="section">
        <h2>Appearance</h2>
        <p class="section-description">Choose an accent color for the interface.</p>
        
        <AccentColorPicker 
          v-model="accentColor"
          @change="onAccentColorChange"
        />
      </div>

      <div class="section">
        <h2>Database Management</h2>
        <div class="stats">
          <p>Total clipboard items: <strong>{{ itemCount }}</strong></p>
          <p>Database size: <strong>{{ formatBytes(databaseSize) }}</strong></p>
        </div>
        
        <div class="actions">
          <button 
            @click="exportDatabase" 
            :disabled="isExporting || itemCount === 0"
            class="action-button export"
          >
            {{ isExporting ? 'Exporting...' : 'Export Database' }}
          </button>
          
          <button 
            @click="importDatabase" 
            :disabled="isImporting"
            class="action-button import"
          >
            {{ isImporting ? 'Importing...' : 'Import Database' }}
          </button>
          
          <button 
            @click="deleteAllData" 
            :disabled="isDeleting || itemCount === 0"
            class="action-button delete"
          >
            {{ isDeleting ? 'Deleting...' : 'Delete All Data' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="less">
.settings {
  font-family: system-ui, sans-serif;
  background: var(--settings-bg-primary);
  height: 100vh;
  display: flex;
  flex-direction: column;
  color: var(--text-primary);
  padding: 8px;
  gap: 8px;
  
  .settings-header {
    background: var(--settings-bg-input);
    border: 1px solid var(--settings-border-color);
    border-radius: 5px;
    padding: 8px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    box-shadow: var(--settings-shadow-light);
    flex-shrink: 0;
    
    .app-info {
      display: flex;
      align-items: center;
      gap: 8px;
      
      .app-icon {
        width: 24px;
        height: 24px;
      }
      
      .app-details {
        h1 {
          margin: 0;
          font-size: 14px;
          font-weight: 600;
          color: var(--text-primary);
        }
        
        .version {
          margin: 1px 0 0 0;
          font-size: 10px;
          color: var(--text-secondary);
        }
      }
    }
    
    .header-actions {
      display: flex;
      align-items: center;
      gap: 6px;
      
      .icon-button {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 24px;
        height: 24px;
        border-radius: 4px;
        color: var(--text-secondary);
        transition: all 0.15s;
        
        &:hover {
          background: var(--settings-bg-primary);
          color: var(--text-primary);
        }
      }
    }
    
    .close-button {
      background: none;
      border: none;
      font-size: 14px;
      cursor: pointer;
      color: var(--text-secondary);
      width: 20px;
      height: 20px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      
      &:hover {
        background: var(--settings-bg-primary);
        color: var(--text-primary);
      }
    }
  }
  
  .settings-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    
    .section {
      background: var(--settings-bg-input);
      border: 1px solid var(--settings-border-color);
      border-radius: 5px;
      padding: 10px 12px;
      box-shadow: var(--settings-shadow-light);
      flex-shrink: 0;
      
      h2 {
        margin: 0 0 8px 0;
        font-size: 12px;
        font-weight: 600;
        color: var(--text-primary);
      }
      
      .path-container {
        margin-bottom: 8px;
        
        .path-label {
          margin: 0 0 2px 0;
          font-size: 11px;
          color: var(--text-secondary);
        }
        
        .path-value {
          margin: 0;
          font-size: 10px;
          font-family: monospace;
          background: rgba(0,0,0,0.05);
          padding: 4px 6px;
          border-radius: 3px;
          word-break: break-all;
          color: var(--text-primary);
          
          &.clickable {
            display: block;
            text-decoration: none;
            cursor: pointer;
            transition: background-color 0.2s;
            
            &:hover {
              background: rgba(0,0,0,0.1);
            }
          }
        }
      }
      
      .stats {
        margin-bottom: 10px;
        
        p {
          margin: 2px 0;
          font-size: 11px;
          color: var(--text-secondary);
          
          strong {
            color: var(--text-primary);
          }
        }
      }
      
      .actions {
        display: flex;
        flex-direction: column;
        gap: 6px;
      }
      
      .section-description {
        margin: 0 0 8px 0 !important;
        font-size: 10px !important;
        color: var(--text-secondary);
      }
      
      .shortcut-config {
        display: flex;
        align-items: center;
        gap: 8px;
      }
      
      .shortcut-display {
        flex: 1;
        background: rgba(0, 0, 0, 0.04);
        border: 1px solid var(--settings-border-color);
        border-radius: 4px;
        padding: 6px 12px;
        text-align: center;
        cursor: pointer;
        transition: all 0.2s;
        
        &:hover {
          background: rgba(0, 0, 0, 0.06);
        }
        
        &.recording {
          background: rgba(32, 178, 170, 0.1);
          border-color: var(--accent);
          box-shadow: 0 0 0 1px rgba(32, 178, 170, 0.2);
        }
        
        .shortcut-keys {
          font-size: 14px;
          font-weight: 500;
          letter-spacing: 1px;
          color: var(--text-primary);
        }
        
        .recording-hint {
          font-size: 10px;
          color: var(--accent);
        }
      }
      
      .action-button.cancel {
        background: rgba(255, 59, 48, 0.1);
        color: #FF3B30;
        border-color: rgba(255, 59, 48, 0.2);
        
        &:hover {
          background: rgba(255, 59, 48, 0.15);
        }
      }
      
      .action-button {
        padding: 6px 12px;
        border: none;
        border-radius: 4px;
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        box-shadow: var(--settings-shadow-light);
        background: var(--settings-bg-primary);
        color: var(--text-primary);
        border: 1px solid var(--settings-border-color);
        
        &:hover {
          filter: brightness(0.95);
        }
        
        &:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        
        &.export {
          background: var(--accent);
          color: var(--accent-text);
          border: none;
          
          &:hover:not(:disabled) {
            filter: brightness(1.1);
            box-shadow: var(--settings-shadow-medium);
          }
        }
        
        &.import {
          background: var(--accent-transparent, rgba(32, 178, 170, 0.25));
          color: var(--accent);
          border: 1px solid var(--accent);
          
          &:hover:not(:disabled) {
            background: var(--accent);
            color: var(--accent-text);
            box-shadow: var(--settings-shadow-medium);
          }
        }
        
        &.delete {
          background: #FF3B30;
          color: white;
          border: none;
          
          &:hover:not(:disabled) {
            background: #D70015;
            box-shadow: var(--settings-shadow-medium);
          }
        }
      }
      
      p {
        line-height: 1.3;
        font-size: 11px;
        color: var(--text-secondary);
        margin: 2px 0;
      }
    }
  }
}
</style>
