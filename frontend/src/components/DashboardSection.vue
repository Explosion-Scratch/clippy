<script setup>
import { ref } from 'vue'

const currentSlide = ref(0)
const slides = [
  {
    caption: 'Comprehensive Clipboard History',
    image: '/screenshots/dashboard-main.png'
  },
  {
    caption: 'Advanced Search & Filtering',
    image: '/screenshots/dashboard-search.png'
  },
  {
    caption: 'Rich Content Previews',
    image: '/screenshots/dashboard-preview.png'
  }
]

const nextSlide = () => {
  currentSlide.value = (currentSlide.value + 1) % slides.length
}

const prevSlide = () => {
  currentSlide.value = (currentSlide.value - 1 + slides.length) % slides.length
}
</script>

<template>
  <section id="dashboard" class="section dashboard-section">
    <div class="container">
      <div class="section-title">
        <h2>Powerful Dashboard</h2>
        <p>Manage your clipboard history with a full-featured web interface.</p>
      </div>

      <div class="dashboard-showcase">
        <div class="dashboard-gallery-wrapper">
          <div class="dashboard-gallery">
            <div class="gallery-frame">
              <img 
                :src="slides[currentSlide].image" 
                :alt="slides[currentSlide].caption"
                class="gallery-image"
              >
            </div>
            <div class="gallery-controls">
              <button class="nav-btn prev" @click="prevSlide" aria-label="Previous slide">
                <i class="ph ph-caret-left"></i>
              </button>
              <span class="gallery-caption">{{ slides[currentSlide].caption }}</span>
              <button class="nav-btn next" @click="nextSlide" aria-label="Next slide">
                <i class="ph ph-caret-right"></i>
              </button>
            </div>
          </div>
        </div>
        <div class="dashboard-features">
          <ul>
            <li>
              <h3>Advanced Search</h3>
              <p>Filter by type, date, and content with regex support.</p>
            </li>
            <li>
              <h3>Bulk Actions</h3>
              <p>Select multiple items to delete or export.</p>
            </li>
            <li>
              <h3>Detailed Previews</h3>
              <p>Inspect HTML, images, and file contents in detail.</p>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dashboard-section {
  padding: 100px 0;
  background: var(--bg-primary);
}

.dashboard-showcase {
  display: grid;
  grid-template-columns: 1.5fr 1fr;
  gap: 60px;
  align-items: center;
  margin-top: 60px;
}

.dashboard-gallery-wrapper {
  display: flex;
  flex-direction: column;
}

.dashboard-gallery {
  background: var(--bg-secondary);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
}

.gallery-frame {
  aspect-ratio: 16/10;
  background: var(--bg-tertiary);
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.gallery-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  /* Visual fallback for broken images during development */
  background: linear-gradient(45deg, var(--bg-secondary) 25%, var(--bg-tertiary) 25%, var(--bg-tertiary) 50%, var(--bg-secondary) 50%, var(--bg-secondary) 75%, var(--bg-tertiary) 75%, var(--bg-tertiary) 100%);
  background-size: 20px 20px;
}

.gallery-controls {
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-light);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.gallery-caption {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  color: var(--text-primary);
  text-align: center;
  flex: 1;
}

.nav-btn {
  background: transparent;
  border: 1px solid transparent;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 8px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.nav-btn:hover {
  background: var(--bg-tertiary);
  color: var(--primary-color);
}

.nav-btn i {
  font-size: 1.25rem;
}

.dashboard-features ul {
  list-style: none;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.dashboard-features h3 {
  font-size: 1.25rem;
  margin-bottom: 8px;
  color: var(--text-primary);
}

.dashboard-features p {
  color: var(--text-secondary);
  line-height: 1.6;
}

@media (max-width: 900px) {
  .dashboard-showcase {
    grid-template-columns: 1fr;
    gap: 40px;
  }
}
</style>
