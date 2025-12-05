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
    icon: 'ph-lightning',
    title: 'Lightning Fast',
    description: 'Native performance with instant search across your entire clipboard history'
  },
  {
    icon: 'ph-magnifying-glass',
    title: 'Powerful Search',
    description: 'Find any item instantly with regex support and smart filtering'
  },
  {
    icon: 'ph-clipboard-text',
    title: 'Rich Content',
    description: 'Full support for text, images, files, HTML, and RTF content'
  },
  {
    icon: 'ph-terminal-window',
    title: 'CLI Power',
    description: 'Complete command-line interface for scripting and automation'
  },
  {
    icon: 'ph-lock-key',
    title: 'Privacy First',
    description: 'All data stored locally on your machine, never in the cloud'
  },
  {
    icon: 'ph-paint-brush',
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
          <a href="https://github.com/Explosion-Scratch/clippy/releases/latest/download/clippy.app.zip" class="btn btn-primary">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M16.145 2.402c0 1.024-.44 2.048-1.062 2.786c-.66.818-1.802 1.426-2.71 1.426c-.104 0-.208-.014-.272-.026a2 2 0 0 1-.038-.362c0-1.038.532-2.062 1.102-2.71c.726-.842 1.93-1.478 2.942-1.516c.026.116.038.26.038.402m3.569 6.45l.05-.033c-1.349-1.933-3.397-1.985-3.967-1.985c-.872 0-1.653.31-2.309.57c-.475.188-.885.35-1.217.35c-.367 0-.787-.17-1.256-.358c-.592-.24-1.26-.51-1.998-.51C6.528 6.886 4 8.95 4 12.837c0 2.424.934 4.979 2.09 6.623C7.088 20.86 7.956 22 9.2 22c.59 0 1.023-.183 1.478-.376c.504-.214 1.036-.44 1.84-.44c.814 0 1.3.212 1.768.416c.437.19.857.374 1.513.374c1.36 0 2.254-1.23 3.11-2.462c.96-1.4 1.363-2.774 1.375-2.838c-.078-.026-2.685-1.077-2.685-4.045c0-2.39 1.777-3.555 2.115-3.777"/></svg>
            Download
          </a>
          <a href="https://github.com/Explosion-Scratch/clippy/releases/latest/download/get_clipboard" class="btn btn-secondary">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 256 256"><path fill="currentColor" d="m117.31 134l-72 64a8 8 0 1 1-10.63-12L100 128L34.69 70a8 8 0 1 1 10.63-12l72 64a8 8 0 0 1 0 12ZM216 184h-96a8 8 0 0 0 0 16h96a8 8 0 0 0 0-16"/></svg>
            Install CLI
          </a>
        </div>
      </div>

      <div class="features-grid">
        <div 
          v-for="(feature, index) in features" 
          :key="feature.title"
          class="feature-card"
          :style="{ animationDelay: `${index * 100}ms` }"
        >
          <i :class="['ph', feature.icon, 'feature-icon']"></i>
          <h3 class="feature-title">{{ feature.title }}</h3>
          <p class="feature-description">{{ feature.description }}</p>
        </div>
      </div>
    </div>
    <svg class="waves" style="height: var(--height);" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1440 320"><path style="width: 100%;"fill="var(--bg-primary)" fill-opacity="1" d="M0,64L13.3,69.3C26.7,75,53,85,80,106.7C106.7,128,133,160,160,165.3C186.7,171,213,149,240,160C266.7,171,293,213,320,208C346.7,203,373,149,400,149.3C426.7,149,453,203,480,197.3C506.7,192,533,128,560,122.7C586.7,117,613,171,640,197.3C666.7,224,693,224,720,197.3C746.7,171,773,117,800,122.7C826.7,128,853,192,880,229.3C906.7,267,933,277,960,240C986.7,203,1013,117,1040,112C1066.7,107,1093,181,1120,192C1146.7,203,1173,149,1200,144C1226.7,139,1253,181,1280,165.3C1306.7,149,1333,75,1360,48C1386.7,21,1413,43,1427,53.3L1440,64L1440,0L1426.7,0C1413.3,0,1387,0,1360,0C1333.3,0,1307,0,1280,0C1253.3,0,1227,0,1200,0C1173.3,0,1147,0,1120,0C1093.3,0,1067,0,1040,0C1013.3,0,987,0,960,0C933.3,0,907,0,880,0C853.3,0,827,0,800,0C773.3,0,747,0,720,0C693.3,0,667,0,640,0C613.3,0,587,0,560,0C533.3,0,507,0,480,0C453.3,0,427,0,400,0C373.3,0,347,0,320,0C293.3,0,267,0,240,0C213.3,0,187,0,160,0C133.3,0,107,0,80,0C53.3,0,27,0,13,0L0,0Z"></path></svg>
  </section>
</template>

<style scoped>
section {
  --height: 160px;
  overflow: visible;
  position: relative;
}
.waves {
  --scale-y: 0.5;
  transform: scaleX(calc(100vw / 710px)) scaleY(var(--scale-y));
  position: absolute;
  right: 0;
  left: 0;
  width: 100vw;
  bottom: calc(calc(-1 * var(--height)) + 2px + calc(var(--height) * var(--scale-y) / 2));
}
.hero {
  padding: 100px 0 80px;
  text-align: center;
  position: relative;
}

.hero-content {
  max-width: 600px;
  margin: 0 auto 60px;
}

.hero-icon-wrapper {
  display: inline-block;
  margin-bottom: 32px;
}

.hero-icon {
  width: 100px;
  height: 100px;
  border-radius: 24px;
  box-shadow: var(--shadow-md);
  transition: transform var(--transition-normal);
}

.hero-icon:hover {
  transform: scale(1.02);
}

.hero-title {
  font-size: 3rem;
  margin: 0 auto;
  margin-bottom: 12px;
  color: var(--text-primary);
  font-weight: 800;
  letter-spacing: 0.02em;
  display: block !important;
  width: fit-content;
}

.hero-tagline {
  font-size: 1.125rem;
  color: var(--text-secondary);
  max-width: 420px;
  margin: 0 auto 32px;
  font-family: var(--font-serif);
  font-style: italic;
}

.hero-cta {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
}

.btn {
  padding: 10px 20px;
  border-radius: 4px;
  font-weight: 500;
  font-size: 0.875rem;
  font-family: var(--font-sans);
  transition: all var(--transition-fast);
  cursor: pointer;
  border: 1px solid transparent;
  letter-spacing: 0.01em;
  display: flex; /* Added for icon alignment */
  align-items: center; /* Added for icon alignment */
  gap: 8px; /* Added for icon spacing */
}

.btn-primary {
  background: var(--accent-dark);
  color: var(--bg-primary);
  border-color: var(--text-primary);
}

.btn-primary:hover {
  background: transparent;
  color: var(--text-primary);
}

.btn-secondary {
  background: transparent;
  color: var(--text-primary);
  border-color: var(--border-light);
}

.btn-secondary:hover {
  border-color: var(--text-primary);
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 32px 48px;
  max-width: 900px;
  margin: 0 auto;
  text-align: left;
}

.feature-card {
  padding: 0;
  background: transparent;
  border: none;
  animation: fadeIn 0.6s ease-out backwards;
}

.feature-icon {
  font-size: 1.25rem;
  display: inline-block;
  margin-bottom: 12px;
  color: var(--text-muted);
}

.feature-title {
  font-size: 1rem;
  margin-bottom: 6px;
  color: var(--text-primary);
  font-weight: 600;
}

.feature-description {
  font-size: 0.875rem;
  color: var(--text-secondary);
  line-height: 1.6;
}

@media (max-width: 768px) {
  .hero {
    padding: 60px 0 50px;
  }

  .hero-title {
    font-size: 2.25rem;
  }

  .hero-tagline {
    font-size: 1rem;
  }

  .hero-icon {
    width: 80px;
    height: 80px;
  }

  .features-grid {
    grid-template-columns: 1fr;
    gap: 24px;
    text-align: center;
  }
  
  .feature-card {
    max-width: 320px;
    margin: 0 auto;
  }
}
</style>
