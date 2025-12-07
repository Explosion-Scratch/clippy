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
      <slot>
        <pre><code>{{ content }}</code></pre>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.terminal-display {
  background: #fafafa;
  border-radius: 10px;
  overflow: hidden;
  width: 100%;
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
  padding: 0;
  min-height: 200px;
  max-height: 400px;
  overflow: hidden;
  line-height: 1;
  background: #fafafa;
  display: flex;
  flex-direction: column;
}

.terminal-content pre {
  margin: 0;
  padding: 16px 20px;
  white-space: pre;
  overflow: auto;
  flex: 1;
  min-height: 0;
}

.terminal-content code {
  font-family: 'Fira Code', 'SF Mono', 'Consolas', monospace;
  font-size: 0.75rem;
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
