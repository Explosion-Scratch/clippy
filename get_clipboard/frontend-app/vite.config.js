import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  base: '/dashboard/',
  server: {
    port: 5173,
    proxy: {
      '/items': 'http://127.0.0.1:3000',
      '/item': 'http://127.0.0.1:3000',
      '/search': 'http://127.0.0.1:3000',
      '/stats': 'http://127.0.0.1:3000',
      '/mtime': 'http://127.0.0.1:3000',
      '/dir': 'http://127.0.0.1:3000',
      '/copy': 'http://127.0.0.1:3000',
      '/save': 'http://127.0.0.1:3000',
    }
  },
  build: {
    outDir: '../frontend-dist',
    emptyOutDir: true,
    assetsDir: 'assets',
  }
})
