<script setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open, ask } from '@tauri-apps/plugin-dialog';
import { writeFile, readFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appVersion = ref('0.1.0');
const itemCount = ref(0);
const databaseSize = ref(0);
const currentDataDir = ref('');
const isDeleting = ref(false);
const isExporting = ref(false);
const isImporting = ref(false);

async function loadStats() {
  try {
    itemCount.value = await invoke('db_get_count');
    databaseSize.value = await invoke('db_get_size');
    currentDataDir.value = await invoke('get_sidecar_dir');
  } catch (error) {
    console.error('Failed to get stats:', error);
  }
}

async function changeDataDirectory() {
  try {
    const selected = await open({
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
    
    const filePath = await open({
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
      await loadStats(); // Refresh the count
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
      await loadStats(); // Refresh the count
    }
  } catch (error) {
    console.error('Failed to delete all data:', error);
    alert('Failed to delete data: ' + error);
  } finally {
    isDeleting.value = false;
  }
}

async function closeSettings() {
  // Just close the window - the native window event handler will restore dock state
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

onMounted(() => {
  loadStats();
  
  // Handle Escape key to close settings window
  document.addEventListener('keyup', async (e) => {
    if (e.key === 'Escape') {
      await closeSettings();
    }
  });
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
      <button @click="closeSettings" class="close-button">×</button>
    </div>

    <div class="settings-content">
      <div class="section">
        <h2>Storage</h2>
        <div class="path-container">
          <p class="path-label">Current Data Directory:</p>
          <p class="path-value">{{ currentDataDir || 'Loading...' }}</p>
        </div>
        <button @click="changeDataDirectory" class="action-button">Change Directory...</button>
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

      <div class="section">
        <h2>About</h2>
        <p>Clippy is a clipboard management application that helps you keep track of your copied items.</p>
        <p>Press Cmd+P to show the clipboard manager, or Cmd+, to open these settings.</p>
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
          background: #34C759;
          color: white;
          border: none;
          
          &:hover:not(:disabled) {
            background: #28A745;
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
