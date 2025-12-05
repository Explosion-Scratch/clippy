<script setup>
import { ref, onMounted } from 'vue'

const isVisible = ref(false)

onMounted(() => {
  setTimeout(() => {
    isVisible.value = true
  }, 100)
})

const features = [
  {
    icon: '⚡',
    title: 'Lightning Fast',
    description: 'Native performance with instant search across your entire clipboard history'
  },
  {
    icon: '🔍',
    title: 'Powerful Search',
    description: 'Find any item instantly with regex support and smart filtering'
  },
  {
    icon: '📋',
    title: 'Rich Content',
    description: 'Full support for text, images, files, HTML, and RTF content'
  },
  {
    icon: '⌨️',
    title: 'CLI Power',
    description: 'Complete command-line interface for scripting and automation'
  },
  {
    icon: '🔒',
    title: 'Privacy First',
    description: 'All data stored locally on your machine, never in the cloud'
  },
  {
    icon: '🎨',
    title: 'Beautiful UI',
    description: 'Clean, minimal interface that stays out of your way'
  }
]
</script>

<template>
  <section class="hero gradient-bg" :class="{ 'fade-in': isVisible }">
    <div class="container">
      <div class="hero-content">
        <div class="hero-icon-wrapper">
          <img src="/icon.png" alt="Clippy" class="hero-icon" />
        </div>
        
        <h1 class="hero-title">Clippy</h1>
        <p class="hero-tagline">
          A minimal yet powerful clipboard manager for macOS
        </p>
        
        <div class="hero-cta">
          <a href="#demo" class="btn btn-primary">Try the Demo</a>
          <a href="#cli" class="btn btn-secondary">Explore CLI</a>
        </div>
      </div>

      <div class="features-grid">
        <div 
          v-for="(feature, index) in features" 
          :key="feature.title"
          class="feature-card"
          :style="{ animationDelay: `${index * 100}ms` }"
        >
          <span class="feature-icon">{{ feature.icon }}</span>
          <h3 class="feature-title">{{ feature.title }}</h3>
          <p class="feature-description">{{ feature.description }}</p>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero {
  padding: 120px 0 100px;
  text-align: center;
  position: relative;
  overflow: hidden;
}

.hero-content {
  max-width: 700px;
  margin: 0 auto 80px;
}

.hero-icon-wrapper {
  display: inline-block;
  margin-bottom: 32px;
  animation: float 6s ease-in-out infinite;
}

.hero-icon {
  width: 120px;
  height: 120px;
  border-radius: 28px;
  box-shadow: var(--shadow-lg), 0 0 0 1px rgba(0,0,0,0.05);
  transition: transform var(--transition-normal);
}

.hero-icon:hover {
  transform: scale(1.05);
}

.hero-title {
  font-size: 4rem;
  margin-bottom: 16px;
  background: linear-gradient(135deg, var(--text-primary) 0%, var(--text-secondary) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero-tagline {
  font-size: 1.375rem;
  color: var(--text-secondary);
  max-width: 500px;
  margin: 0 auto 40px;
  font-family: var(--font-serif);
  font-style: italic;
}

.hero-cta {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
}

.btn {
  padding: 14px 28px;
  border-radius: 8px;
  font-weight: 500;
  font-size: 1rem;
  transition: all var(--transition-fast);
  cursor: pointer;
  border: none;
}

.btn-primary {
  background: var(--text-primary);
  color: var(--bg-primary);
}

.btn-primary:hover {
  background: var(--text-secondary);
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.btn-secondary {
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-light);
}

.btn-secondary:hover {
  background: var(--bg-tertiary);
  transform: translateY(-2px);
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
  max-width: 960px;
  margin: 0 auto;
}

.feature-card {
  background: var(--bg-primary);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 28px;
  text-align: left;
  transition: all var(--transition-normal);
  animation: fadeIn 0.6s ease-out backwards;
}

.feature-card:hover {
  border-color: var(--border-light);
  box-shadow: var(--shadow-md);
  transform: translateY(-4px);
}

.feature-icon {
  font-size: 2rem;
  display: block;
  margin-bottom: 16px;
}

.feature-title {
  font-size: 1.125rem;
  margin-bottom: 8px;
  color: var(--text-primary);
}

.feature-description {
  font-size: 0.9375rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

@media (max-width: 768px) {
  .hero {
    padding: 80px 0 60px;
  }

  .hero-title {
    font-size: 2.5rem;
  }

  .hero-tagline {
    font-size: 1.125rem;
  }

  .hero-icon {
    width: 100px;
    height: 100px;
  }

  .features-grid {
    grid-template-columns: 1fr;
  }
}
</style>
