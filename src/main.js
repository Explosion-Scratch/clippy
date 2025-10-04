import { createApp } from "vue";
import App from "./App.vue";
import ClipboardItem from "./components/ClipboardItem.vue";
import router from "./router";

const app = createApp(App);
app.component("ClipboardItem", ClipboardItem);
app.use(router);
app.mount("#app");
