import { createApp } from "vue";
import { createPinia } from "pinia";
import { i18n } from "./i18n";
import { APP_SETTINGS_STORAGE_KEY, useAppStore } from "./app/appStore";
import { useTheme } from "./shared/composables/useTheme";
import ReferenceWindowPage from "./features/reference/ReferenceWindowPage.vue";
import "./shared/styles/globals.scss";

const pinia = createPinia();
const appStore = useAppStore(pinia);
const { applyAppearance } = useTheme();

appStore.hydrate();

function applyStoredAppearance() {
  appStore.hydrate();
  applyAppearance(appStore.appearance);
  i18n.global.locale.value = appStore.language;
}

applyStoredAppearance();

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === APP_SETTINGS_STORAGE_KEY) applyStoredAppearance();
  });
}

createApp(ReferenceWindowPage).use(pinia).use(i18n).mount("#app");
