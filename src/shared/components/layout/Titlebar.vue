<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { availableMonitors, getCurrentWindow, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";
import { Copy, Maximize, Minimize, X } from "lucide-vue-next";
import { useAppStore, type MainWindowGeometry } from "../../../app/appStore";
import AppTooltip from "../ui/AppTooltip.vue";
import logoUrl from "../../../../logos/logo,transparent.png";

const appStore = useAppStore();
const isMaximized = ref(false);
const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let appWindow: ReturnType<typeof getCurrentWindow> | undefined;
const emit = defineEmits<{
  "maximized-change": [value: boolean];
}>();

let unlistenResize: (() => void) | undefined;
let unlistenMove: (() => void) | undefined;
let geometrySaveTimer: ReturnType<typeof window.setTimeout> | undefined;
let isRestoringGeometry = false;
let disposed = false;

async function syncMaximizedState() {
  if (!appWindow) return;

  isMaximized.value = await appWindow.isMaximized();
  emit("maximized-change", isMaximized.value);
}

async function minimizeWindow() {
  if (!appWindow) return;
  await appWindow.minimize();
}

async function toggleMaximize() {
  if (!appWindow) return;
  await appWindow.toggleMaximize();
  await syncMaximizedState();
}

async function closeWindow() {
  if (!appWindow) return;
  await appWindow.close();
}

function startWindowDrag(event: PointerEvent) {
  if (event.button !== 0 || !appWindow) return;

  event.preventDefault();
  void appWindow.startDragging().catch(() => undefined);
}

async function restoreWindowGeometry() {
  if (!appWindow) return;

  isRestoringGeometry = true;
  try {
    const geometry = appStore.mainWindowGeometry;
    if (!geometry || !await isGeometryVisible(geometry)) {
      await appWindow.center();
      return;
    }

    await appWindow.setSize(new PhysicalSize(geometry.width, geometry.height));
    await appWindow.setPosition(new PhysicalPosition(geometry.x, geometry.y));
    if (geometry.isMaximized) await appWindow.maximize();
  } catch {
    // Keep the configuration-provided centered geometry when native restoration is unavailable.
  } finally {
    isRestoringGeometry = false;
  }
}

async function isGeometryVisible(geometry: MainWindowGeometry) {
  try {
    const monitors = await availableMonitors();
    return monitors.some((monitor) => {
      const left = Math.max(geometry.x, monitor.workArea.position.x);
      const top = Math.max(geometry.y, monitor.workArea.position.y);
      const right = Math.min(geometry.x + geometry.width, monitor.workArea.position.x + monitor.workArea.size.width);
      const bottom = Math.min(geometry.y + geometry.height, monitor.workArea.position.y + monitor.workArea.size.height);

      return right - left >= 120 && bottom - top >= 90;
    });
  } catch {
    return false;
  }
}

function scheduleGeometrySave() {
  if (isRestoringGeometry) return;
  if (geometrySaveTimer !== undefined) window.clearTimeout(geometrySaveTimer);

  geometrySaveTimer = window.setTimeout(() => {
    geometrySaveTimer = undefined;
    void saveWindowGeometry();
  }, 150);
}

async function saveWindowGeometry() {
  if (!appWindow || isRestoringGeometry) return;

  try {
    const [position, size, maximized] = await Promise.all([
      appWindow.outerPosition(),
      appWindow.innerSize(),
      appWindow.isMaximized(),
    ]);
    const previous = appStore.mainWindowGeometry;

    if (maximized) {
      if (previous) appStore.mainWindowGeometry = { ...previous, isMaximized: true };
      return;
    }

    appStore.mainWindowGeometry = {
      x: position.x,
      y: position.y,
      width: size.width,
      height: size.height,
      isMaximized: false,
    };
  } catch {
    // Retain the last valid geometry if the host denies an OS-level window query.
  }
}

onMounted(() => {
  if (!isTauriEnvironment) return;

  appWindow = getCurrentWindow();
  void restoreWindowGeometry().then(() => {
    void syncMaximizedState();
    scheduleGeometrySave();
  });
  void appWindow.onResized(() => {
    void syncMaximizedState();
    scheduleGeometrySave();
  }).then((stopListening) => {
    if (disposed) {
      stopListening();
      return;
    }

    unlistenResize = stopListening;
  });
  void appWindow.onMoved(() => {
    scheduleGeometrySave();
  }).then((stopListening) => {
    if (disposed) {
      stopListening();
      return;
    }

    unlistenMove = stopListening;
  });
});

onUnmounted(() => {
  disposed = true;
  unlistenResize?.();
  unlistenMove?.();
  if (geometrySaveTimer !== undefined) window.clearTimeout(geometrySaveTimer);
  void saveWindowGeometry();
});
</script>

<template>
  <header class="titlebar iv-glass-surface">
    <div class="titlebar-brand" @pointerdown="startWindowDrag">
      <img class="brand-mark" :src="logoUrl" alt="" aria-hidden="true">
      <span class="titlebar-name">{{ $t("app.name") }}</span>
    </div>

    <div
      class="titlebar-drag-region"
      aria-hidden="true"
      @pointerdown="startWindowDrag"
      @dblclick="toggleMaximize"
    />

    <div class="window-controls">
      <AppTooltip :content="$t('window.minimize')">
        <button class="window-control" type="button" :aria-label="$t('window.minimize')" @click="minimizeWindow">
          <Minimize :size="16" :stroke-width="1.8" aria-hidden="true" />
        </button>
      </AppTooltip>
      <AppTooltip :content="isMaximized ? $t('window.restore') : $t('window.maximize')">
        <button
          class="window-control"
          type="button"
          :aria-label="isMaximized ? $t('window.restore') : $t('window.maximize')"
          @click="toggleMaximize"
        >
          <Copy v-if="isMaximized" :size="16" :stroke-width="1.8" aria-hidden="true" />
          <Maximize v-else :size="16" :stroke-width="1.8" aria-hidden="true" />
        </button>
      </AppTooltip>
      <AppTooltip :content="$t('window.close')">
        <button
          class="window-control close-control"
          type="button"
          :aria-label="$t('window.close')"
          @click="closeWindow"
        >
          <X :size="17" :stroke-width="1.8" aria-hidden="true" />
        </button>
      </AppTooltip>
    </div>
  </header>
</template>

<style scoped lang="scss">
.titlebar {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  min-width: 0;
  min-height: var(--iv-titlebar-height);
  color: var(--iv-text-primary);
  background-color: var(--iv-chrome-surface);
  border-bottom: 1px solid var(--iv-border-subtle);
  z-index: var(--iv-layer-shell);
}

.titlebar-brand,
.titlebar-drag-region {
  height: 100%;
  user-select: none;
}

.titlebar-brand {
  display: flex;
  align-items: center;
  gap: var(--iv-space-3);
  min-width: 0;
  padding: 0 var(--iv-space-4);
}

.titlebar-drag-region {
  min-width: 0;
}

.brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  object-fit: contain;
  letter-spacing: 0;
}

.titlebar-name {
  min-width: 0;
  overflow: hidden;
  font-size: var(--iv-font-size-14);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.window-controls {
  display: flex;
  align-items: center;
  height: 100%;
}

.window-control {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  color: var(--iv-text-secondary);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.window-control:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.window-control:active {
  transform: scale(var(--iv-press-scale));
}

.close-control:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-danger-hover);
}
</style>
