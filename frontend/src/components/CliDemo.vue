<script setup>
import { ref, computed, watch } from 'vue'
import TerminalDisplay from './TerminalDisplay.vue'
import cliExamples from '../data/cli-examples.json'

const selectedCommand = ref('history')
const selectedSubcommand = ref(null)
const selectedFlags = ref([])

const commandMeta = {
  history: {
    description: 'Show clipboard history',
    subcommands: null,
    flags: ['--limit', '--json', '--text', '--image', '--file', '--html', '--help']
  },
  show: {
    description: 'Show a specific clipboard item',
    subcommands: null,
    flags: ['--json', '--text', '--image', '--file', '--html', '--help']
  },
  search: {
    description: 'Search clipboard history',
    subcommands: null,
    flags: ['--limit', '--regex', '--sort', '--json', '--help']
  },
  copy: {
    description: 'Copy an item to the system clipboard',
    subcommands: null,
    flags: ['--help']
  },
  paste: {
    description: 'Copy an item and paste it',
    subcommands: null,
    flags: ['--help']
  },
  delete: {
    description: 'Delete an item from history',
    subcommands: null,
    flags: ['--help']
  },
  service: {
    description: 'Manage the clipboard watcher service',
    subcommands: ['status', 'start', 'stop', 'install', 'uninstall', 'logs'],
    flags: ['--help']
  },
  dir: {
    description: 'Manage data directory',
    subcommands: ['get', 'set', 'move'],
    flags: ['--help']
  },
  stats: {
    description: 'Show clipboard statistics',
    subcommands: null,
    flags: ['--json', '--help']
  },
  export: {
    description: 'Export clipboard history',
    subcommands: null,
    flags: ['--help']
  },
  import: {
    description: 'Import clipboard history',
    subcommands: null,
    flags: ['--help']
  },
  interactive: {
    description: 'Launch the TUI (Terminal User Interface)',
    subcommands: null,
    flags: ['--help']
  },
  permissions: {
    description: 'Check or request accessibility permissions',
    subcommands: ['check', 'request'],
    flags: ['--help']
  }
}

const commands = computed(() => {
  const result = {}
  for (const [name, meta] of Object.entries(commandMeta)) {
    result[name] = {
      ...meta,
      examples: cliExamples[name] || {}
    }
  }
  return result
})

const commandNames = Object.keys(commandMeta)

const currentExamples = computed(() => {
  const cmd = commands.value[selectedCommand.value]
  if (!cmd) return {}
  return cmd.examples
})

const currentExample = computed(() => {
  const cmd = commands.value[selectedCommand.value]
  if (!cmd) return null
  
  let key = selectedSubcommand.value || 'default'
  if (selectedFlags.value.length > 0) {
    const flagKey = selectedFlags.value.join(' ').replace(/ /g, '_')
    if (cmd.examples[flagKey]) {
      key = flagKey
    } else if (selectedSubcommand.value && cmd.examples[`${selectedSubcommand.value}_${flagKey}`]) {
      key = `${selectedSubcommand.value}_${flagKey}`
    }
  }
  
  return cmd.examples[key] || cmd.examples.default || null
})

const currentOutput = computed(() => {
  const example = currentExample.value
  if (!example) return ''

  // Strip ANSI color escape codes (e.g., \u001b[32m, \u001b[0m)
  const rawOutput = Array.isArray(example) ? example[1] : example
  if (!rawOutput) return ''
  
  // Regex to match ANSI escape codes
  // eslint-disable-next-line no-control-regex
  return rawOutput.replace(/\x1b\[[0-9;]*m/g, '')
})

const currentCommandStr = computed(() => {
  if (!currentExample.value) {
    // Fallback if no example found, though this shouldn't happen often
    let str = `get_clipboard ${selectedCommand.value}`
    if (selectedSubcommand.value) {
      str += ` ${selectedSubcommand.value}`
    }
    if (selectedFlags.value.length > 0) {
      str += ` ${selectedFlags.value.join(' ')}`
    }
    return str
  }

  let cmdStr = currentExample.value[0]

  // If the user didn't select --limit, remove it from the display
  // because we often use it internally for keeping examples short
  if (!selectedFlags.value.includes('--limit')) {
    // Remove --limit <number> and trim extra spaces
    cmdStr = cmdStr.replace(/\s*--limit\s+\d+/, '')
  }

  return cmdStr.trim()
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
  // Single flag selection logic:
  // If the clicked flag is already selected, deselect it.
  // Otherwise, select ONLY the clicked flag (clearing others).
  const idx = selectedFlags.value.indexOf(flag)
  if (idx >= 0) {
    selectedFlags.value = []
  } else {
    selectedFlags.value = [flag]
  }
}
</script>

<template>
  <section id="cli" class="section cli-demo-section">
    <div class="container">
      <div class="section-title">
        <h2>Command Line Interface</h2>
        <p>
          The core of Clippy is through its CLI. Script, grep, automate, whatever! It's all private, fast and open source.
          <a href="https://github.com/explosion-scratch/clippy/tree/main/get_clipboard/README.md">(See docs)</a>
        </p>
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
            <code class="current-command">
              <span class="prompt">$</span> 
              <span class="cmd-text">{{ currentCommandStr }}</span>
            </code>
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

          <div class="terminal-wrapper">
             <TerminalDisplay v-if="!currentOutput && commands[selectedCommand]?.subcommands">
               <div class="empty-state-actions">
                 <p class="empty-hint">Select a subcommand to view usage examples:</p>
                 <div class="quick-actions">
                   <button 
                     v-for="sub in commands[selectedCommand].subcommands" 
                     :key="sub"
                     @click="selectSubcommand(sub)"
                     class="action-btn"
                   >
                    <span class="cmd-prefix">$ {{ selectedCommand }}</span>
                     <span class="cmd-name">{{ sub }}</span>
                     <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="arrow-icon"><line x1="5" y1="12" x2="19" y2="12"></line><polyline points="12 5 19 12 12 19"></polyline></svg>
                   </button>
                 </div>
               </div>
             </TerminalDisplay>
             <TerminalDisplay v-else :content="currentOutput" />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.terminal-wrapper {
  padding: 0 16px 16px 16px;
  display: flex;
  height: 100%;
}
.cli-demo-section {
  background: var(--bg-primary);
}

.cli-explorer {
  display: grid;
  grid-template-columns: 250px 1fr;
  gap: 32px;
  background: var(--bg-secondary);
  border-radius: 16px;
  padding: 24px;
  box-shadow: var(--shadow-md), 0 6px 40px -5px var(--accent-transparent);
  border: 2px solid var(--accent-transparent);
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
  gap: 1px;
}

.command-list li {
  padding: 6px 10px;
  border-radius: 4px;
  cursor: pointer;
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  gap: 8px;
}

.command-list li:hover {
  background: var(--bg-tertiary);
}

.command-list li.active {
  background: var(--accent);
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
  font-size: 0.8125rem;
  font-weight: 500;
}

.command-desc {
  font-size: 0.6875rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-detail {
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
  display: flex;
  align-items: center;
  gap: 10px;
}

.prompt {
  color: var(--primary-color);
  opacity: 0.7;
  user-select: none;
}

.cmd-text {
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
  background: var(--accent-dark);
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

.empty-state-actions {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
  justify-content: center;
  padding: 12px 0;
}

.empty-hint {
  font-size: 0.875rem;
  color: var(--text-muted);
  text-align: center;
  margin: 0;
}

.quick-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 12px;
  max-width: 600px;
  margin: 0 auto;
  width: 100%;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
}

.action-btn:hover {
  background: var(--bg-tertiary);
  border-color: var(--primary-color);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.cmd-prefix, .cmd-name {
  font-family: var(--font-mono);
}

.cmd-prefix {
  font-size: 0.75rem;
  color: var(--text-muted);
  opacity: 0.7;
}

.cmd-name {
  font-size: 0.9375rem;
  font-weight: 500;
  color: var(--text-primary);
  flex: 1;
}

.arrow-icon {
  color: var(--text-muted);
  transition: transform 0.2s ease;
}

.action-btn:hover .arrow-icon {
  transform: translateX(3px);
  color: var(--primary-color);
}

</style>
