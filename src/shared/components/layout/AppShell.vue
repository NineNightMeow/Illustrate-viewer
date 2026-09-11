<script setup lang="ts">
import { ref } from "vue";
import { useAppStore } from "../../../app/appStore";
import OverlayRoot from "./OverlayRoot.vue";
import Sidebar from "./Sidebar.vue";
import Titlebar from "./Titlebar.vue";
import Topbar from "./Topbar.vue";

const appStore = useAppStore();
const isMaximized = ref(false);
</script>

<template>
  <div
    class="app-shell"
    :class="{
      'is-maximized': isMaximized,
      'is-sidebar-collapsed': appStore.sidebarCollapsed,
    }"
  >
    <Titlebar @maximized-change="isMaximized = $event" />

    <div class="app-main">
      <Sidebar />

      <section class="workspace" :aria-label="$t('workspace.label')">
        <Topbar />
        <main class="workspace-content">
          <RouterView />
        </main>
      </section>
    </div>

    <OverlayRoot />
  </div>
</template>

<style scoped lang="scss">
.app-shell {
  --iv-sidebar-width: var(--iv-sidebar-expanded-width);

  display: grid;
  grid-template-rows: var(--iv-titlebar-height) minmax(0, 1fr);
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  color: var(--iv-text-primary);
  background-color: var(--iv-app-background);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-xl);
}

.app-shell.is-maximized {
  border-color: transparent;
  border-radius: 0;
}

.app-shell.is-sidebar-collapsed {
  --iv-sidebar-width: var(--iv-sidebar-collapsed-width);
}

.app-main {
  display: grid;
  grid-template-columns: var(--iv-sidebar-width) minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  transition: grid-template-columns var(--iv-motion-normal) ease;
}

.workspace {
  display: grid;
  grid-template-rows: var(--iv-topbar-height) minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  background-color: var(--iv-content-surface);
}

.workspace-content {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  background-color: var(--iv-content-surface);
}
</style>
