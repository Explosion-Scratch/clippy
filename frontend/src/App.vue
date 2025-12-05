<script setup>
import HeroSection from './components/HeroSection.vue'
import ClipboardDemo from './components/ClipboardDemo.vue'
import CliDemo from './components/CliDemo.vue'
import FooterSection from './components/FooterSection.vue'
</script>

<template>
  <div class="app subtle-grain">
    <HeroSection />
    <ClipboardDemo />
    <CliDemo />
    <FooterSection />
  </div>
</template>

<style>
.app {
  min-height: 100vh;
}
</style>
