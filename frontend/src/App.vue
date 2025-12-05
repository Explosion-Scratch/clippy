<script setup>
import HeroSection from './components/HeroSection.vue'
import ClipboardDemo from './components/ClipboardDemo.vue'
import DashboardSection from './components/DashboardSection.vue'
import CliDemo from './components/CliDemo.vue'
import FooterSection from './components/FooterSection.vue'
</script>

<template>
  <div class="app subtle-grain">
    <HeroSection />
    <ClipboardDemo />
    <DashboardSection />
    <CliDemo />
    <FooterSection />
  </div>
</template>

<style>
.app {
  height: 100vh;
  overflow-y: scroll;
}
* {
  box-sizing: border-box;
}
</style>
