# Marketing Website (frontend/)

You are a Vue 3 specialist working on Clippy's marketing landing page.

## Project Knowledge

- **Tech Stack:** Vue 3 (Composition API), Vite, Tailwind CSS
- **Purpose:** Public-facing marketing website with demos
- **Entry:** `index.html` → `src/main.js` → `src/App.vue`

### Directory Structure

| Path | Purpose |
|------|---------|
| `src/App.vue` | Main layout |
| `src/main.js` | Vue bootstrap |
| `src/style.css` | Global styles + Tailwind |
| `src/components/` | Marketing components |
| `src/data/` | Demo data |
| `scripts/` | Build scripts for demo content |

## Commands

```bash
cd frontend
bun install               # Install dependencies
bun run dev               # Development server (port 5173)
bun run build             # Production build to dist/
```

## Code Style

### Marketing Component
```vue
<script setup>
import { ref, onMounted } from 'vue'

const isVisible = ref(false)

onMounted(() => {
  const observer = new IntersectionObserver(entries => {
    isVisible.value = entries[0].isIntersecting
  })
  observer.observe(document.querySelector('.hero'))
})
</script>

<template>
  <section class="hero min-h-screen flex items-center justify-center">
    <div class="text-center">
      <h1 class="text-5xl font-bold text-gray-900 dark:text-white">
        Clippy
      </h1>
      <p class="mt-4 text-xl text-gray-600 dark:text-gray-300">
        Your clipboard, reimagined.
      </p>
    </div>
  </section>
</template>
```

### Animation Pattern
```vue
<template>
  <div 
    class="transition-all duration-700"
    :class="isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'"
  >
    <!-- Content -->
  </div>
</template>
```

## Components

| Component | Purpose |
|-----------|---------|
| `HeroSection.vue` | Landing hero with tagline |
| `CliDemo.vue` | Interactive CLI demonstration |
| `ClipboardDemo.vue` | Animated clipboard preview |
| `DashboardSection.vue` | Dashboard feature showcase |
| `ScreenshotGallery.vue` | App screenshots carousel |
| `FooterSection.vue` | Links and footer |

## Conventions

- **Tailwind First**: Use utility classes, avoid custom CSS
- **Dark Mode**: Support system preference with `dark:` variants
- **Animations**: Use Tailwind transitions and IntersectionObserver
- **Demo Data**: Generate from `scripts/`, don't hardcode

## Boundaries

- ✅ **Always do:**
  - Use Tailwind utilities
  - Support dark mode
  - Optimize images
  - Make sections responsive

- ⚠️ **Ask first:**
  - Adding new sections
  - Changing branding/colors
  - Adding tracking/analytics

- 🚫 **Never do:**
  - Use external fonts (use Tailwind defaults)
  - Add heavy JavaScript libraries
  - Include real user data
