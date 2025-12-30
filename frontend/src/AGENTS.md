# Marketing Website Source

You are a Vue 3 specialist working on the marketing site's source code.

## Project Knowledge

- **Purpose:** Landing page source files
- **Pattern:** Simple SPA without routing

### File Structure

| File | Purpose |
|------|---------|
| `App.vue` | Root component, page sections |
| `main.js` | Vue initialization |
| `style.css` | Tailwind + custom styles |
| `components/` | Section components |
| `data/` | Demo data files |
| `assets/` | Images, icons |

## Code Style

### App Layout
```vue
<!-- App.vue -->
<script setup>
import HeroSection from './components/HeroSection.vue'
import CliDemo from './components/CliDemo.vue'
import DashboardSection from './components/DashboardSection.vue'
import FooterSection from './components/FooterSection.vue'
</script>

<template>
  <div class="min-h-screen bg-gray-50 dark:bg-gray-900">
    <HeroSection />
    <CliDemo />
    <DashboardSection />
    <FooterSection />
  </div>
</template>
```

### Style Setup
```css
/* style.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

/* Custom component classes */
@layer components {
  .section-container {
    @apply max-w-7xl mx-auto px-4 sm:px-6 lg:px-8;
  }
}
```

## Conventions

- **Section Components**: Each page section is a component
- **No Router**: Single page, scroll-based navigation
- **Tailwind Layers**: Use `@layer components` for reusable classes
- **Dark Mode**: Use Tailwind's dark mode (class-based)

## Boundaries

- ✅ **Always do:**
  - Keep App.vue as simple section composition
  - Use Tailwind layers for shared styles
  - Support mobile and desktop

- ⚠️ **Ask first:**
  - Adding routing
  - Major layout changes

- 🚫 **Never do:**
  - Put logic in main.js
  - Add heavy animations libraries
