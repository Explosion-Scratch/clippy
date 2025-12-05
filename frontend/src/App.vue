<script setup>
import HeroSection from './components/HeroSection.vue'
import ClipboardDemo from './components/ClipboardDemo.vue'
import DashboardSection from './components/DashboardSection.vue'
import ScreenshotGallery from './components/ScreenshotGallery.vue'
import CliDemo from './components/CliDemo.vue'
import FooterSection from './components/FooterSection.vue'
</script>

<template>
  <div class="app">
    <HeroSection />
    <ClipboardDemo />
    <DashboardSection />
    <ScreenshotGallery />
    <CliDemo />
    <FooterSection />
  </div>
</template>

<style>
.app {
  height: 100vh;
  overflow-y: scroll;
  width: 100vw;
  overflow-x: hidden;
}
* {
  box-sizing: border-box;
}
</style>
