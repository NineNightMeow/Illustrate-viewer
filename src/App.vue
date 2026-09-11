<script setup lang="ts">
import { watch } from "vue";
import { useAppStore } from "./app/appStore";
import AppShell from "./shared/components/layout/AppShell.vue";
import { i18n } from "./i18n";
import { useTheme } from "./shared/composables/useTheme";
import CustomBackgroundLayer from "./app/CustomBackgroundLayer.vue";

const appStore = useAppStore();
const { applyAppearance } = useTheme();

appStore.hydrate();

watch(
  () => ({
    appearance: appStore.appearance,
    language: appStore.language,
    sidebarCollapsed: appStore.sidebarCollapsed,
    mainWindowGeometry: appStore.mainWindowGeometry,
  }),
  ({ appearance, language }) => {
    applyAppearance(appearance);
    i18n.global.locale.value = language;
    appStore.persist();
  },
  { deep: true, immediate: true },
);
</script>

<template>
  <div
    class="app-root"
    :class="{
      'has-custom-background': appStore.appearance.customBackground.enabled
        && appStore.customBackgroundSourceStatus === 'available',
    }"
  >
    <CustomBackgroundLayer
      :enabled="appStore.appearance.customBackground.enabled"
      :source="appStore.appearance.customBackground.source"
      :blur="appStore.appearance.customBackground.blur"
      @source-status="appStore.setCustomBackgroundSourceStatus($event)"
    />
    <AppShell />
  </div>
</template>

<style lang="scss">
.app-root {
  position: relative;
  isolation: isolate;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: var(--iv-app-background);
}

.app-root > .app-shell {
  position: relative;
  z-index: 1;
}

.app-root.has-custom-background {
  --iv-content-surface: color-mix(in srgb, var(--iv-bg-primary) 20%, transparent);
  --iv-chrome-surface: color-mix(in srgb, var(--iv-bg-primary) 56%, transparent);
  --iv-chrome-sidebar-surface: color-mix(in srgb, var(--iv-bg-secondary) 56%, transparent);
}

.app-root.has-custom-background > .app-shell {
  background-color: transparent;
}
</style>
