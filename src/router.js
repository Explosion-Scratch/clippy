import { createRouter, createWebHistory } from 'vue-router';
import ClipboardManager from './components/ClipboardManager.vue';
import Settings from './components/Settings.vue';

const routes = [
  {
    path: '/',
    name: 'clipboard',
    component: ClipboardManager
  },
  {
    path: '/settings',
    name: 'settings',
    component: Settings
  }
];

const router = createRouter({
  history: createWebHistory(),
  routes
});

export default router;