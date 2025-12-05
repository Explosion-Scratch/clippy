<script setup>
import { ref, computed, watch } from 'vue'
import TerminalDisplay from './TerminalDisplay.vue'

const selectedCommand = ref('history')
const selectedSubcommand = ref(null)
const selectedFlags = ref([])

const commands = {
  history: {
    description: 'Show clipboard history',
    subcommands: null,
    flags: ['--limit', '--query', '--from', '--to', '--sort', '--json', '--full', '--text', '--image', '--file'],
    examples: {
      default: `0 (0) [11:23 x3]   async function fetchUserData(userId) { ... }
1 (1) [11:21 x1]   https://github.com/example/clippy
2 (2) [11:18 x2]   [Image: 245.0K]
3 (3) [11:15 x5]   The quick brown fox jumps over the lazy dog...
4 (4) [11:10 x1]   [Files: 2 items]
5 (5) [11:05 x2]   <div class="container">...</div>
6 (6) [11:00 x4]   npm install @clippy/core --save-dev
7 (7) [10:45 x1]   Remember to update the API documentation...`,
      '--limit 3': `0 (0) [11:23 x3]   async function fetchUserData(userId) { ... }
1 (1) [11:21 x1]   https://github.com/example/clippy
2 (2) [11:18 x2]   [Image: 245.0K]`,
      '--query api': `3 (3) [11:15 x5]   Remember to update the API documentation...
7 (7) [10:45 x4]   npm install @clippy/core --save-dev`,
      '--json --limit 2': `[
  {
    "hash": "a1b2c3d4",
    "offset": 0,
    "summary": "async function fetchUserData...",
    "type": "text",
    "byteSize": 142,
    "copyCount": 3,
    "lastSeen": "2024-01-15T11:23:45Z"
  },
  {
    "hash": "e5f6g7h8",
    "offset": 1,
    "summary": "https://github.com/example/clippy",
    "type": "text",
    "byteSize": 35,
    "copyCount": 1,
    "lastSeen": "2024-01-15T11:21:30Z"
  }
]`,
      '--text': `0 (0) [11:23 x3]   async function fetchUserData(userId) { ... }
1 (1) [11:21 x1]   https://github.com/example/clippy
2 (2) [11:15 x5]   The quick brown fox jumps over the lazy dog...
3 (3) [11:00 x4]   npm install @clippy/core --save-dev`,
      '--image': `0 (0) [11:18 x2]   [Image: 245.0K - Screenshot 2024-01-15.png]`,
      '--sort copies': `0 (3) [11:15 x5]   The quick brown fox jumps over the lazy dog...
1 (6) [11:00 x4]   npm install @clippy/core --save-dev
2 (0) [11:23 x3]   async function fetchUserData(userId) { ... }
3 (2) [11:18 x2]   [Image: 245.0K]`
    }
  },
  show: {
    description: 'Show a specific clipboard item',
    subcommands: null,
    flags: ['--json', '--text', '--image', '--file', '--html'],
    examples: {
      default: `async function fetchUserData(userId) {
  const response = await fetch(\`/api/users/\${userId}\`);
  return response.json();
}

──────────────────────────────────────────
Type: text  Size: 142 bytes  Copies: 3
First copied: 2024-01-15 11:23:45`,
      '1': `https://github.com/example/clippy

──────────────────────────────────────────
Type: text  Size: 35 bytes  Copies: 1
First copied: 2024-01-15 11:21:30`,
      '--json 0': `{
  "hash": "a1b2c3d4",
  "offset": 0,
  "summary": "async function fetchUserData(userId) {...}",
  "type": "text",
  "byteSize": 142,
  "copyCount": 3,
  "timestamp": "2024-01-15T10:15:30Z",
  "lastSeen": "2024-01-15T11:23:45Z",
  "data": {
    "text": "async function fetchUserData(userId) {\\n  const response = await fetch(\`/api/users/\${userId}\`);\\n  return response.json();\\n}"
  }
}`
    }
  },
  search: {
    description: 'Search clipboard history',
    subcommands: null,
    flags: ['--limit', '--regex', '--sort', '--json'],
    examples: {
      default: `Usage: get_clipboard search <QUERY> [OPTIONS]

Search clipboard history by content.

Arguments:
  <QUERY>  Search query string

Options:
  -l, --limit <N>    Maximum number of results
      --regex        Treat query as regex pattern
      --sort <SORT>  Sort order: date, copies, type, relevance
      --json         Output as JSON`,
      'function': `0 (0) [11:23 x3]   async function fetchUserData(userId) { ... }`,
      'api --sort relevance': `0 (3) [11:15 x5]   Remember to update the API documentation...
1 (7) [10:45 x4]   npm install @clippy/core --save-dev`,
      '"^https?://" --regex': `0 (1) [11:21 x1]   https://github.com/example/clippy`
    }
  },
  copy: {
    description: 'Copy an item to the system clipboard',
    subcommands: null,
    flags: [],
    examples: {
      default: `Copied: async function fetchUserData(userId) { ... }`,
      '3': `Copied: The quick brown fox jumps over the lazy dog...`
    }
  },
  paste: {
    description: 'Copy an item and paste it',
    subcommands: null,
    flags: [],
    examples: {
      default: `Copied: async function fetchUserData(userId) { ... }
[Simulating Cmd+V...]`,
      '2': `Copied: [Image: 245.0K]
[Simulating Cmd+V...]`
    }
  },
  delete: {
    description: 'Delete an item from history',
    subcommands: null,
    flags: [],
    examples: {
      default: `Deleted: async function fetchUserData(userId) { ... }`,
      '5': `Deleted: <div class="container">...</div>`
    }
  },
  service: {
    description: 'Manage the clipboard watcher service',
    subcommands: ['status', 'start', 'stop', 'install', 'uninstall', 'logs'],
    flags: [],
    examples: {
      'status': `Service: get_clipboard
Status: running
PID: 12345
Uptime: 2h 34m 12s
Installed: true`,
      'start': `Starting clipboard service...
Service started successfully.`,
      'stop': `Stopping clipboard service...
Service stopped.`,
      'install': `Installing launchd service...
Created: ~/Library/LaunchAgents/com.clippy.agent.plist
Service installed successfully.`,
      'uninstall': `Uninstalling launchd service...
Removed: ~/Library/LaunchAgents/com.clippy.agent.plist
Service uninstalled.`,
      'logs': `[2024-01-15 11:23:45] INFO  Stored text entry: a1b2c3d4
[2024-01-15 11:21:30] INFO  Stored text entry: e5f6g7h8
[2024-01-15 11:18:22] INFO  Stored image entry: i9j0k1l2
[2024-01-15 11:15:10] INFO  Stored text entry: m3n4o5p6
[2024-01-15 11:10:05] INFO  Stored files entry: q7r8s9t0`,
      'logs -n 3 --follow': `[2024-01-15 11:23:45] INFO  Stored text entry: a1b2c3d4
[2024-01-15 11:21:30] INFO  Stored text entry: e5f6g7h8
[2024-01-15 11:18:22] INFO  Stored image entry: i9j0k1l2
[Watching for new entries...]`
    }
  },
  dir: {
    description: 'Manage data directory',
    subcommands: ['get', 'set', 'move'],
    flags: [],
    examples: {
      'get': `/Users/demo/Library/Application Support/Clippy`,
      'set /path/to/new/dir': `Data directory set to: /path/to/new/dir`,
      'move /path/to/new/dir': `Moving data directory...
Moved 847 items to: /path/to/new/dir
Data directory updated.`
    }
  },
  stats: {
    description: 'Show clipboard statistics',
    subcommands: null,
    flags: ['--json'],
    examples: {
      default: `Clipboard Statistics
====================
Total items:    847
Reported size:  12.3 MB
Storage size:   15.7 MB

By type:
  text       623
  image      156
  file        68

Top 20 Largest Items (by storage):
Index    Type       Size         Summary
----------------------------------------------------------------------
2        image      2.4M         Screenshot 2024-01-15 at 10.32.45 AM
15       image      1.8M         Design mockup v3.png
4        file       1.2M         [Files: 2 items]
...`,
      '--json': `{
  "total_items": 847,
  "total_size": 12890234,
  "actual_storage_size": 16472891,
  "type_counts": {
    "text": 623,
    "image": 156,
    "file": 68
  },
  "largest_items": [
    {
      "hash": "i9j0k1l2",
      "kind": "image",
      "storage_size": 2516582,
      "summary": "Screenshot 2024-01-15 at 10.32.45 AM"
    }
  ]
}`
    }
  },
  export: {
    description: 'Export clipboard history',
    subcommands: null,
    flags: [],
    examples: {
      'backup.json': `Exporting 847 items...
  Processed 100/847 items
  Processed 200/847 items
  ...
  Processed 847/847 items
Exported 847 items to backup.json`
    }
  },
  import: {
    description: 'Import clipboard history',
    subcommands: null,
    flags: [],
    examples: {
      'backup.json': `Importing from version 1.0.0 (847 items)...
  [1/847] Imported: async function fetchUserData...
  [2/847] Imported: https://github.com/example/clippy
  ...
  [523/847] Skipped (exists): npm install @clippy/core
  ...
Import complete: 324 imported, 523 skipped, 0 errors`
    }
  },
  interactive: {
    description: 'Launch the TUI (Terminal User Interface)',
    subcommands: null,
    flags: ['--query'],
    examples: {
      default: `[See interactive.txt for TUI preview]`
    }
  },
  permissions: {
    description: 'Check or request accessibility permissions',
    subcommands: ['check', 'request'],
    flags: [],
    examples: {
      'check': `Accessibility permissions granted`,
      'request': `Opened System Settings to request permissions`
    }
  }
}

const commandNames = Object.keys(commands)

const currentExamples = computed(() => {
  const cmd = commands[selectedCommand.value]
  if (!cmd) return {}
  return cmd.examples
})

const currentOutput = computed(() => {
  const cmd = commands[selectedCommand.value]
  if (!cmd) return ''
  
  let key = selectedSubcommand.value || 'default'
  if (selectedFlags.value.length > 0) {
    const flagKey = selectedFlags.value.join(' ')
    if (cmd.examples[flagKey]) {
      key = flagKey
    } else if (selectedSubcommand.value && cmd.examples[`${selectedSubcommand.value} ${flagKey}`]) {
      key = `${selectedSubcommand.value} ${flagKey}`
    }
  }
  
  return cmd.examples[key] || cmd.examples.default || ''
})

const currentCommandStr = computed(() => {
  let str = `get_clipboard ${selectedCommand.value}`
  if (selectedSubcommand.value) {
    str += ` ${selectedSubcommand.value}`
  }
  if (selectedFlags.value.length > 0) {
    str += ` ${selectedFlags.value.join(' ')}`
  }
  return str
})

function selectCommand(cmd) {
  selectedCommand.value = cmd
  selectedSubcommand.value = null
  selectedFlags.value = []
}

function selectSubcommand(sub) {
  selectedSubcommand.value = sub
  selectedFlags.value = []
}

function toggleFlag(flag) {
  const idx = selectedFlags.value.indexOf(flag)
  if (idx >= 0) {
    selectedFlags.value.splice(idx, 1)
  } else {
    selectedFlags.value.push(flag)
  }
}
</script>

<template>
  <section id="cli" class="section cli-demo-section">
    <div class="container">
      <div class="section-title">
        <h2>Command Line Interface</h2>
        <p>Full-featured CLI for scripting and power users. Explore all commands below.</p>
      </div>

      <div class="cli-explorer">
        <div class="command-sidebar">
          <h4>Commands</h4>
          <ul class="command-list">
            <li 
              v-for="cmd in commandNames" 
              :key="cmd"
              :class="{ active: selectedCommand === cmd }"
              @click="selectCommand(cmd)"
            >
              <code>{{ cmd }}</code>
              <span class="command-desc">{{ commands[cmd].description }}</span>
            </li>
          </ul>
        </div>

        <div class="command-detail">
          <div class="command-header">
            <code class="current-command">$ {{ currentCommandStr }}</code>
          </div>

          <div v-if="commands[selectedCommand]?.subcommands" class="subcommands">
            <span class="label">Subcommands:</span>
            <button 
              v-for="sub in commands[selectedCommand].subcommands"
              :key="sub"
              :class="{ active: selectedSubcommand === sub }"
              @click="selectSubcommand(sub)"
            >
              {{ sub }}
            </button>
          </div>

          <div v-if="commands[selectedCommand]?.flags?.length" class="flags">
            <span class="label">Flags:</span>
            <button 
              v-for="flag in commands[selectedCommand].flags"
              :key="flag"
              :class="{ active: selectedFlags.includes(flag) }"
              @click="toggleFlag(flag)"
            >
              {{ flag }}
            </button>
          </div>

          <TerminalDisplay :content="currentOutput" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.cli-demo-section {
  background: var(--bg-primary);
}

.cli-explorer {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 32px;
  background: var(--bg-secondary);
  border-radius: 16px;
  padding: 24px;
  box-shadow: var(--shadow-md);
}

.command-sidebar h4 {
  font-family: var(--font-sans);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.command-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.command-list li {
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all var(--transition-fast);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.command-list li:hover {
  background: var(--bg-tertiary);
}

.command-list li.active {
  background: var(--text-primary);
  color: var(--bg-primary);
}

.command-list li.active code {
  color: var(--bg-primary);
}

.command-list li.active .command-desc {
  color: var(--bg-tertiary);
}

.command-list code {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  font-weight: 500;
}

.command-desc {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.command-detail {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.command-header {
  padding: 12px 16px;
  background: var(--bg-tertiary);
  border-radius: 8px;
}

.current-command {
  font-family: var(--font-mono);
  font-size: 0.9375rem;
  color: var(--text-primary);
}

.subcommands,
.flags {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.label {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 500;
}

.subcommands button,
.flags button {
  padding: 6px 12px;
  border: 1px solid var(--border-light);
  background: var(--bg-primary);
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.subcommands button:hover,
.flags button:hover {
  border-color: var(--text-secondary);
}

.subcommands button.active,
.flags button.active {
  background: var(--text-primary);
  color: var(--bg-primary);
  border-color: var(--text-primary);
}

@media (max-width: 900px) {
  .cli-explorer {
    grid-template-columns: 1fr;
  }

  .command-list {
    flex-direction: row;
    flex-wrap: wrap;
    gap: 8px;
  }

  .command-list li {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .command-desc {
    display: none;
  }
}
</style>
