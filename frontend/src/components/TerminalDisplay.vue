<script setup>
const props = defineProps({
  content: {
    type: String,
    default: ''
  }
})
</script>

<template>
  <div class="terminal-display">
    <div class="terminal-header">
      <div class="traffic-lights">
        <span class="dot close"></span>
        <span class="dot minimize"></span>
        <span class="dot maximize"></span>
      </div>
      <span class="terminal-title">Terminal</span>
    </div>
    <div class="terminal-content">
      <pre><code>{{ content }}</code></pre>
    </div>
  </div>
</template>

<style scoped>
.terminal-display {
  background: #fafafa;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 
    0 0 0 1px rgba(0, 0, 0, 0.06),
    0 4px 12px rgba(0, 0, 0, 0.08);
}

.terminal-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: #f0f0f0;
  border-bottom: 1px solid #e5e5e5;
}

.traffic-lights {
  display: flex;
  gap: 6px;
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.dot.close { background: #ff5f56; }
.dot.minimize { background: #ffbd2e; }
.dot.maximize { background: #27c93f; }

.terminal-title {
  font-size: 0.75rem;
  color: #888;
  font-weight: 500;
}

.terminal-content {
  padding: 16px 20px;
  min-height: 200px;
  max-height: 400px;
  overflow-y: auto;
  background: #fafafa;
}

.terminal-content pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.terminal-content code {
  font-family: 'Fira Code', 'SF Mono', 'Consolas', monospace;
  font-size: 0.8125rem;
  line-height: 1.6;
  color: #1a1a1a;
  font-feature-settings: "liga" 1, "calt" 1;
}

@media (prefers-color-scheme: dark) {
  .terminal-display {
    background: #fafafa;
  }
  
  .terminal-content code {
    color: #1a1a1a;
  }
}
</style>
