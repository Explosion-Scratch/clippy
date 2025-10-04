<script setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeFile, readFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appVersion = ref('0.1.0');
const itemCount = ref(0);
const isDeleting = ref(false);
const isExporting = ref(false);
const isImporting = ref(false);

async function loadStats() {
  try {
    itemCount.value = await invoke('db_get_count');
  } catch (error) {
    console.error('Failed to get item count:', error);
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
  if (confirm('Are you sure you want to delete all clipboard items? This action cannot be undone.')) {
    try {
      isDeleting.value = true;
      const result = await invoke('db_delete_all');
      alert(result);
      await loadStats(); // Refresh the count
    } catch (error) {
      console.error('Failed to delete all data:', error);
      alert('Failed to delete data: ' + error);
    } finally {
      isDeleting.value = false;
    }
  }
}

async function closeSettings() {
  const window = getCurrentWindow();
  await window.close();
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
        <img src="/tauri.svg" alt="App Icon" class="app-icon" />
        <div class="app-details">
          <h1>Clippy Settings</h1>
          <p class="version">Version {{ appVersion }}</p>
        </div>
      </div>
      <button @click="closeSettings" class="close-button">×</button>
    </div>

    <div class="settings-content">
      <div class="section">
        <h2>Database Management</h2>
        <div class="stats">
          <p>Total clipboard items: <strong>{{ itemCount }}</strong></p>
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
  font-family: system-ui, -apple-system, sans-serif;
  background: #f5f5f5;
  height: 100vh;
  display: flex;
  flex-direction: column;
  
  .settings-header {
    background: white;
    padding: 12px;
    border-bottom: 1px solid #e0e0e0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    
    .app-info {
      display: flex;
      align-items: center;
      gap: 10px;
      
      .app-icon {
        width: 32px;
        height: 32px;
      }
      
      .app-details {
        h1 {
          margin: 0;
          font-size: 18px;
          font-weight: 600;
          color: #333;
        }
        
        .version {
          margin: 2px 0 0 0;
          font-size: 11px;
          color: #666;
        }
      }
    }
    
    .close-button {
      background: none;
      border: none;
      font-size: 18px;
      cursor: pointer;
      color: #666;
      width: 24px;
      height: 24px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      
      &:hover {
        background: #e0e0e0;
      }
    }
  }
  
  .settings-content {
    flex: 1;
    padding: 12px;
    overflow-y: auto;
    
    .section {
      background: white;
      border-radius: 6px;
      padding: 14px;
      margin-bottom: 12px;
      
      h2 {
        margin: 0 0 10px 0;
        font-size: 15px;
        font-weight: 600;
        color: #333;
      }
      
      .stats {
        margin-bottom: 12px;
        
        p {
          margin: 0;
          font-size: 12px;
          color: #666;
          
          strong {
            color: #333;
          }
        }
      }
      
      .actions {
        display: flex;
        flex-direction: column;
        gap: 8px;
        
        .action-button {
          padding: 8px 14px;
          border: none;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s ease;
          
          &:disabled {
            opacity: 0.5;
            cursor: not-allowed;
          }
          
          &.export {
            background: #007AFF;
            color: white;
            
            &:hover:not(:disabled) {
              background: #0056CC;
            }
          }
          
          &.import {
            background: #34C759;
            color: white;
            
            &:hover:not(:disabled) {
              background: #28A745;
            }
          }
          
          &.delete {
            background: #FF3B30;
            color: white;
            
            &:hover:not(:disabled) {
              background: #D70015;
            }
          }
        }
      }
      
      p {
        line-height: 1.4;
        font-size: 12px;
        color: #666;
        margin: 4px 0;
      }
    }
  }
}
</style>