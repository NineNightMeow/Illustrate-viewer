import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { i18n } from "./i18n";
import router from "./app/routes";
import "./shared/styles/globals.scss";

createApp(App).use(createPinia()).use(router).use(i18n).mount("#app");
