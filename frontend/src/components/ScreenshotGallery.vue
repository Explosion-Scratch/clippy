<script setup>
import { ref, onMounted } from 'vue'

const slides = [
  {
    image: '/clippy/images/app/color.png',
    title: 'Sleek, Minimal, Usable',
    description: 'A distraction-free interface designed for focus and efficiency.'
  },
  {
    image: '/clippy/images/app/files.png',
    title: 'File Management',
    description: 'Handle files directly from your clipboard history.'
  },
  {
    image: '/clippy/images/app/settings.png',
    title: 'Powerful Configuration',
    description: 'Customize every aspect of your experience.'
  },
  {
    image: '/clippy/images/app/tray.png',
    title: 'Always Available',
    description: 'Access your clipboard instantly from the menu bar.'
  }
]

const currentIndex = ref(0)
const isTransitioning = ref(false)

const nextSlide = () => {
  if (isTransitioning.value) return
  isTransitioning.value = true
  currentIndex.value = (currentIndex.value + 1) % slides.length
  setTimeout(() => isTransitioning.value = false, 800)
}

const prevSlide = () => {
  if (isTransitioning.value) return
  isTransitioning.value = true
  currentIndex.value = (currentIndex.value - 1 + slides.length) % slides.length
  setTimeout(() => isTransitioning.value = false, 800)
}
</script>

<template>
  <section class="screenshot-gallery">
    <div class="gallery-container">
      <div 
        v-for="(slide, index) in slides" 
        :key="index"
        class="slide"
        :class="{ active: index === currentIndex }"
      >
        <div class="image-wrapper">
          <img :src="slide.image" :alt="slide.title" />
          <div class="overlay"></div>
        </div>
        <div class="caption-container">
          <h2 class="slide-title">{{ slide.title }}</h2>
          <p class="slide-description">{{ slide.description }}</p>
        </div>
      </div>

      <button class="nav-btn prev" @click="prevSlide" aria-label="Previous slide">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor" width="32" height="32">
          <path d="M165.66,202.34a8,8,0,0,1-11.32,11.32l-80-80a8,8,0,0,1,0-11.32l80-80a8,8,0,0,1,11.32,11.32L91.31,128Z"/>
        </svg>
      </button>
      <button class="nav-btn next" @click="nextSlide" aria-label="Next slide">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor" width="32" height="32">
          <path d="M181.66,133.66l-80,80a8,8,0,0,1-11.32-11.32L164.69,128,90.34,53.66a8,8,0,0,1,11.32-11.32l80,80A8,8,0,0,1,181.66,133.66Z"/>
        </svg>
      </button>

      <div class="indicators">
        <div 
          v-for="(_, index) in slides" 
          :key="index"
          class="dot"
          :class="{ active: index === currentIndex }"
          @click="currentIndex = index"
        ></div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.screenshot-gallery {
  width: 100%;
  height: 80vh;
  min-height: 600px;
  position: relative;
  background: var(--bg-secondary);
  overflow: hidden;
  font-family: var(--font-sans);
}

.gallery-container {
  width: 100%;
  height: 100%;
  position: relative;
}

.slide {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  /* When becoming inactive (leaving): delay opacity change until new slide covers it */
  transition: opacity 0s linear 0.8s;
  z-index: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  pointer-events: none;
}

.slide.active {
  opacity: 1;
  pointer-events: auto;
  z-index: 2;
  /* When becoming active (entering): fade in immediately on top */
  transition: opacity 0.8s ease-in-out;
}

.image-wrapper {
  width: 100%;
  height: 100%;
  position: relative;
}

.image-wrapper img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center top; 
}

.overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 50%;
  background: linear-gradient(to top, rgba(0,0,0,0.8), transparent);
  pointer-events: none;
}

.caption-container {
  position: absolute;
  bottom: 80px;
  left: 50%;
  transform: translateX(-50%);
  text-align: center;
  z-index: 20;
  width: 90%;
  max-width: 800px;
  color: white;
  text-shadow: 0 2px 10px rgba(0,0,0,0.3);
}

.slide-title {
  font-size: 3rem;
  font-weight: 800;
  margin-bottom: 1rem;
  letter-spacing: -0.02em;
  background: linear-gradient(135deg, #fff 0%, #e0e0e0 100%);
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  width: fit-content;
  margin-left: auto;
  margin-right: auto;
}

.slide-title::after {
  content: none;
}

.slide-description {
  font-size: 1.25rem;
  color: rgba(255, 255, 255, 0.9);
  max-width: 600px;
  margin: 0 auto;
  font-weight: 400;
}

.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  background: #0003;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: white;
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 30;
  transition: all 0.3s ease;
  backdrop-filter: blur(10px);
  font-size: 1.5rem;
}

.nav-btn:hover {
  background: rgba(0, 0, 0, 0.4);
  transform: translateY(-50%) scale(1.05);
}

.nav-btn.prev {
  left: 32px;
}

.nav-btn.next {
  right: 32px;
}

.indicators {
  position: absolute;
  bottom: 32px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 12px;
  z-index: 30;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  cursor: pointer;
  transition: all 0.3s ease;
}

.dot.active {
  background: white;
  transform: scale(1.2);
}

@media (max-width: 768px) {
  .slide-title {
    font-size: 2rem;
  }
  
  .slide-description {
    font-size: 1rem;
  }
  
  .nav-btn {
    width: 40px;
    height: 40px;
    font-size: 1.2rem;
  }
  
  .nav-btn.prev { left: 16px; }
  .nav-btn.next { right: 16px; }
}
</style>
