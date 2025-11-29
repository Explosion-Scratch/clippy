import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  base: '/dashboard/',
  server: {
    port: 5173,
    proxy: {
      '/items': 'http://127.0.0.1:3016',
      '/item': 'http://127.0.0.1:3016',
      '/search': 'http://127.0.0.1:3016',
      '/stats': 'http://127.0.0.1:3016',
      '/mtime': 'http://127.0.0.1:3016',
      '/dir': 'http://127.0.0.1:3016',
      '/copy': 'http://127.0.0.1:3016',
      '/save': 'http://127.0.0.1:3016',
    }
  },
  build: {
    rollupOptions: {
      input: 'index.html',
    },
    outDir: '../frontend-dist',
    emptyOutDir: true,
    assetsDir: 'assets',
  }
})
