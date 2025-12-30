# Dashboard Source

You are a Vue 3 specialist working on the dashboard's source code.

## Project Knowledge

- **Purpose:** Source files for the web dashboard
- **Pattern:** Standard Vue 3 SPA structure

### File Structure

| File | Purpose |
|------|---------|
| `App.vue` | Root component, layout, routing |
| `main.js` | Vue app initialization |
| `style.css` | Global Tailwind imports |
| `components/` | UI components |
| `composables/` | Reactive logic |
| `assets/` | Static resources |

## Code Style

### App Structure
```vue
<!-- App.vue -->
<script setup>
import { ref } from 'vue'
import Sidebar from './components/Sidebar.vue'
import TopBar from './components/TopBar.vue'
import ItemList from './components/ItemList.vue'
import ItemDetail from './components/ItemDetail.vue'

const selectedItem = ref(null)
</script>

<template>
  <div class="app-container">
    <Sidebar />
    <main class="main-content">
      <TopBar />
      <div class="content-area">
        <ItemList @select="selectedItem = $event" />
        <ItemDetail v-if="selectedItem" :item="selectedItem" />
      </div>
    </main>
  </div>
</template>
```

### Main Entry
```javascript
// main.js
import { createApp } from 'vue'
import App from './App.vue'
import './style.css'

createApp(App).mount('#app')
```

### Global Styles
```css
/* style.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

/* Custom utilities if needed */
```

## Conventions

- **Single Page**: No vue-router; App.vue manages all state
- **Composables**: Shared logic in `composables/`
- **Tailwind**: Global styles minimal, use utilities in templates

## Boundaries

- ✅ **Always do:**
  - Keep `main.js` minimal
  - Use Tailwind for styling
  - Handle component state in App.vue

- ⚠️ **Ask first:**
  - Adding vue-router
  - Adding state management (Pinia)

- 🚫 **Never do:**
  - Add heavy dependencies
  - Put business logic in main.js
