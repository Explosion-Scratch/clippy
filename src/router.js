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
  },
  {
    path: '/preview',
    name: 'preview',
    component: () => import('./components/PreviewWindow.vue')
  },
  {
    path: '/welcome',
    name: 'welcome',
    component: () => import('./components/Welcome.vue')
  }
];

const router = createRouter({
  history: createWebHistory(),
  routes
});

export default router;