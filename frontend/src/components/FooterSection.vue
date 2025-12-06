<script setup>
</script>

<template>
  <footer class="footer">
    <div class="container">
      <div class="footer-content">
        <div class="footer-brand">
          <img src="/icon.png" alt="Clippy" class="footer-icon" />
          <span class="footer-name">Clippy</span>
        </div>
        <p class="footer-tagline">Tiny but powerful clipboard manager for macOS</p>
        <div class="footer-links">
          <a href="https://github.com/Explosion-Scratch/clippy" target="_blank">GitHub</a>
          <span class="divider">·</span>
          <a href="https://github.com/Explosion-Scratch" target="_blank">@Explosion-Scratch</a>
        </div>
        <p class="footer-copyright">Built with ❤️ for clipboard power users</p>
      </div>
    </div>
  </footer>
</template>

<style scoped>
.footer {
  padding: 60px 0 40px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-subtle);
}

.footer-content {
  text-align: center;
}

.footer-brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 12px;
}

.footer-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
}

.footer-name {
  font-family: var(--font-serif);
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text-primary);
}

.footer-tagline {
  font-size: 0.9375rem;
  color: var(--text-secondary);
  margin-bottom: 20px;
}

.footer-links {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 24px;
}

.footer-links a {
  font-size: 0.875rem;
  color: var(--text-secondary);
  transition: color var(--transition-fast);
}

.footer-links a:hover {
  color: var(--text-primary);
}

.divider {
  color: var(--text-muted);
}

.footer-copyright {
  font-size: 0.8125rem;
  color: var(--text-muted);
}
</style>
