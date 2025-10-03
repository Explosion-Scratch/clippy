import { createApp } from "vue";
import App from "./App.vue";
import ClipboardItem from "./components/ClipboardItem.vue";

const app = createApp(App);
app.component("ClipboardItem", ClipboardItem);
app.mount("#app");
