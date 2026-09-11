<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";
import { Droplets, Ellipsis, FlipHorizontal, Lock, Pin, Unlock, X } from "lucide-vue-next";
import AppContextMenu from "../../shared/components/ui/AppContextMenu.vue";
import AppTooltip from "../../shared/components/ui/AppTooltip.vue";
import {
  IMAGE_PAN_OVERSCROLL,
  calculateActualSizeZoom,
  calculateBaseFitScale,
  calculateCursorCenteredZoom,
  calculateFitZoom,
  calculateMaximumZoom,
  calculateRenderedImageSize,
  clampImagePan,
} from "../../shared/imageViewport";
import { useReferenceWindowStore, type ReferenceWindowState } from "./referenceWindowStore";
import { shortcutDispatcher } from "../../shared/shortcuts/shortcutDispatcher";
import { isPointerInteractiveTarget } from "../../shared/utils/isPointerInteractiveTarget";

type ReferenceAsset = {
  imageId: string;
  filename: string;
  sourcePath: string;
  state: ReferenceWindowState;
};

const { t } = useI18n();
const referenceStore = useReferenceWindowStore();
const { mirrored, opacity, alwaysOnTop, locked } = storeToRefs(referenceStore);
const asset = ref<ReferenceAsset | null>(null);
const isLoading = ref(true);
const hasError = ref(false);
const imageFailed = ref(false);
const surface = ref<HTMLElement | null>(null);
const imageElement = ref<HTMLImageElement | null>(null);
const baseFitScale = ref<number | null>(null);
const zoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const isPanning = ref(false);
const isTopmostUpdating = ref(false);
const isLockUpdating = ref(false);
const opacityPopoverOpen = ref(false);
const moreMenuOpen = ref(false);
const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const imageSource = computed(() => {
  if (!asset.value?.sourcePath) return null;
  return isTauriEnvironment ? convertFileSrc(asset.value.sourcePath) : asset.value.sourcePath;
});
const showError = computed(() => hasError.value || imageFailed.value);
const errorMessage = computed(() => imageFailed.value
  ? t("viewer.referenceImageFailed")
  : t("viewer.referenceRestoreFailed"));
const fitZoom = computed(() => getFitZoom());
const isZoomed = computed(() => zoom.value > fitZoom.value + 0.01);
const imageTransform = computed(() =>
  "translate3d(" + panX.value + "px, " + panY.value + "px, 0) scale(" + zoom.value + ")"
  + (mirrored.value ? " scaleX(-1)" : ""),
);
const imageOpacity = computed(() => opacity.value / 100);
const contextMenuItems = computed(() => [
  { value: "mirror", label: t(mirrored.value ? "viewer.unmirror" : "viewer.mirror") },
  { value: "opacity", label: t("viewer.opacity", { value: opacity.value }) },
  {
    value: "always-on-top",
    label: t("viewer.alwaysOnTop") + ": " + t(alwaysOnTop.value ? "settings.on" : "settings.off"),
    disabled: isTopmostUpdating.value,
  },
  { value: "fit", label: t("viewer.fit") },
  { value: "actual-size", label: t("viewer.actualSize") },
  { value: "lock", label: t(locked.value ? "viewer.unlock" : "viewer.lock"), disabled: isLockUpdating.value },
]);

let resizeObserver: ResizeObserver | null = null;
let unregisterShortcuts: (() => void) | null = null;
let dragState: {
  pointerId: number;
  startX: number;
  startY: number;
  panX: number;
  panY: number;
} | null = null;

onMounted(() => {
  void loadReference();
  unregisterShortcuts = shortcutDispatcher.register("reference", {
    "reference.fit": () => { resetFit(); },
    "reference.actualSize": () => { setActualSize(); },
  });
  resizeObserver = new ResizeObserver(updateFit);
  if (surface.value) resizeObserver.observe(surface.value);
});

onBeforeUnmount(() => {
  unregisterShortcuts?.();
  resizeObserver?.disconnect();
});

async function loadReference() {
  try {
    const reference = await invoke<ReferenceAsset>("get_reference_asset");
    asset.value = reference;
    referenceStore.hydrate(reference.state);
    await syncAlwaysOnTop();
  } catch {
    hasError.value = true;
  } finally {
    isLoading.value = false;
  }
}

async function syncAlwaysOnTop() {
  if (!isTauriEnvironment) return;

  try {
    referenceStore.setAlwaysOnTop(await getCurrentWindow().isAlwaysOnTop());
  } catch {
    // The window starts non-topmost when the host does not expose this state.
  }
}

function toggleMirror() {
  referenceStore.toggleMirror();
  moreMenuOpen.value = false;
  void saveReferenceState();
}

function setOpacityFromInput(event: Event) {
  const input = event.target;
  if (!(input instanceof HTMLInputElement)) return;
  referenceStore.setOpacity(Number(input.value));
}

function cycleOpacity() {
  referenceStore.setOpacity(opacity.value === 100 ? 75 : opacity.value <= 25 ? 100 : opacity.value - 25);
  void saveReferenceState();
}

async function toggleAlwaysOnTop() {
  if (isTopmostUpdating.value) return;

  const nextValue = !alwaysOnTop.value;
  if (!isTauriEnvironment) {
    referenceStore.setAlwaysOnTop(nextValue);
    void saveReferenceState();
    return;
  }

  isTopmostUpdating.value = true;
  try {
    await getCurrentWindow().setAlwaysOnTop(nextValue);
    referenceStore.setAlwaysOnTop(nextValue);
    void saveReferenceState();
  } catch {
    // Keep the toolbar state aligned with the unchanged native state.
  } finally {
    isTopmostUpdating.value = false;
  }
}

async function toggleLock() {
  if (isLockUpdating.value) return;

  const nextValue = !locked.value;
  moreMenuOpen.value = false;
  if (!isTauriEnvironment) {
    referenceStore.setLocked(nextValue);
    void saveReferenceState();
    return;
  }

  isLockUpdating.value = true;
  try {
    await getCurrentWindow().setResizable(!nextValue);
    referenceStore.setLocked(nextValue);
    void saveReferenceState();
  } catch {
    // Keep the menu state aligned with the unchanged native resize state.
  } finally {
    isLockUpdating.value = false;
  }
}

function toggleOpacityPopover() {
  opacityPopoverOpen.value = !opacityPopoverOpen.value;
  moreMenuOpen.value = false;
}

function toggleMoreMenu() {
  moreMenuOpen.value = !moreMenuOpen.value;
  opacityPopoverOpen.value = false;
}

function handleContextSelect(value: string) {
  if (value === "mirror") toggleMirror();
  else if (value === "opacity") cycleOpacity();
  else if (value === "always-on-top") void toggleAlwaysOnTop();
  else if (value === "fit") resetFit();
  else if (value === "actual-size") setActualSize();
  else if (value === "lock") void toggleLock();
}

async function saveReferenceState() {
  if (!isTauriEnvironment) return;

  try {
    await invoke("update_reference_state", {
      referenceState: {
        mirrored: mirrored.value,
        opacity: opacity.value,
        alwaysOnTop: alwaysOnTop.value,
        locked: locked.value,
      },
    });
  } catch {
    // The next state change or shutdown flush can retry a transient write failure.
  }
}

function handleImageLoad() {
  updateFit();
}

function updateFit() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight) return;

  const wasAtFit = Math.abs(zoom.value - fitZoom.value) < 0.01;
  baseFitScale.value = calculateBaseFitScale(
    surface.value.clientWidth,
    surface.value.clientHeight,
    image.naturalWidth,
    image.naturalHeight,
  );
  const nextFitZoom = fitZoom.value;
  if (wasAtFit) zoom.value = nextFitZoom;
  else if (zoom.value < nextFitZoom) zoom.value = nextFitZoom;
  setClampedPan(panX.value, panY.value, false);
}

function getFitZoom() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight) return 1;

  const normalFit = baseFitScale.value ?? calculateBaseFitScale(
    surface.value.clientWidth,
    surface.value.clientHeight,
    image.naturalWidth,
    image.naturalHeight,
  );
  return calculateFitZoom({
    surfaceWidth: surface.value.clientWidth,
    surfaceHeight: surface.value.clientHeight,
    imageWidth: image.naturalWidth,
    imageHeight: image.naturalHeight,
    baseFitScale: normalFit,
    rotation: 0,
  });
}

function getImageMetrics() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight || baseFitScale.value === null) return null;

  return calculateRenderedImageSize({
    imageWidth: image.naturalWidth,
    imageHeight: image.naturalHeight,
    baseFitScale: baseFitScale.value,
    zoom: zoom.value,
    rotation: 0,
  });
}

function setClampedPan(nextPanX: number, nextPanY: number, allowOverscroll: boolean) {
  const metrics = getImageMetrics();
  if (!surface.value || !metrics) {
    panX.value = nextPanX;
    panY.value = nextPanY;
    return;
  }

  const clamped = clampImagePan({
    panX: nextPanX,
    panY: nextPanY,
    surfaceWidth: surface.value.clientWidth,
    surfaceHeight: surface.value.clientHeight,
    imageWidth: metrics.width,
    imageHeight: metrics.height,
    overscroll: allowOverscroll ? IMAGE_PAN_OVERSCROLL : 0,
  });
  panX.value = clamped.panX;
  panY.value = clamped.panY;
}

function zoomAt(clientX: number, clientY: number, requestedZoom: number) {
  if (!surface.value || baseFitScale.value === null) return;

  const bounds = surface.value.getBoundingClientRect();
  const next = calculateCursorCenteredZoom({
    currentZoom: zoom.value,
    requestedZoom,
    minimumZoom: fitZoom.value,
    maximumZoom: calculateMaximumZoom(baseFitScale.value),
    panX: panX.value,
    panY: panY.value,
    clientX,
    clientY,
    surfaceLeft: bounds.left,
    surfaceTop: bounds.top,
    surfaceWidth: bounds.width,
    surfaceHeight: bounds.height,
  });
  if (!next) return;

  zoom.value = next.zoom;
  setClampedPan(next.panX, next.panY, false);
}

function resetFit() {
  zoom.value = fitZoom.value;
  panX.value = 0;
  panY.value = 0;
}

function setActualSize() {
  if (baseFitScale.value === null) return;

  zoom.value = calculateActualSizeZoom(baseFitScale.value, fitZoom.value);
  panX.value = 0;
  panY.value = 0;
}

function handlePointerDown(event: PointerEvent) {
  if (isPointerInteractiveTarget(event.target)) return;

  if (event.button === 0 && (event.altKey || !isZoomed.value)) {
    event.preventDefault();
    void startWindowDrag();
    return;
  }

  const canPanWithLeftButton = event.button === 0 && isZoomed.value;
  if (event.button !== 1 && !canPanWithLeftButton) return;
  if (!surface.value) return;

  event.preventDefault();
  dragState = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    panX: panX.value,
    panY: panY.value,
  };
  isPanning.value = true;
  surface.value.setPointerCapture(event.pointerId);
}

function handlePointerMove(event: PointerEvent) {
  if (!dragState || event.pointerId !== dragState.pointerId) return;

  setClampedPan(
    dragState.panX + event.clientX - dragState.startX,
    dragState.panY + event.clientY - dragState.startY,
    true,
  );
}

function handlePointerEnd(event: PointerEvent) {
  if (!dragState || event.pointerId !== dragState.pointerId) return;

  if (surface.value?.hasPointerCapture(event.pointerId)) surface.value.releasePointerCapture(event.pointerId);
  dragState = null;
  isPanning.value = false;
  setClampedPan(panX.value, panY.value, false);
}

async function startWindowDrag() {
  try {
    if (isTauriEnvironment && !locked.value) await getCurrentWindow().startDragging();
  } catch {
    // Keep the image usable if native dragging is unavailable.
  }
}

function handleWheel(event: WheelEvent) {
  if (event.deltaY === 0) return;
  event.preventDefault();

  const delta = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * 16 : event.deltaY;
  zoomAt(event.clientX, event.clientY, zoom.value * Math.exp(-delta * 0.0015));
}

async function closeWindow() {
  try {
    await saveReferenceState();
    if (isTauriEnvironment) await getCurrentWindow().close();
    else window.close();
  } catch {
    // Keep the reference image visible if the host refuses to close it.
  }
}
</script>

<template>
  <main class="reference-window" :aria-label="asset?.filename ?? $t('viewer.reference')">
    <AppContextMenu :items="contextMenuItems" @select="handleContextSelect">
      <template #trigger>
        <section
          ref="surface"
          class="reference-window__surface"
          :class="{ 'is-zoomed': isZoomed, 'is-panning': isPanning }"
          @wheel="handleWheel"
          @pointerdown="handlePointerDown"
          @pointermove="handlePointerMove"
          @pointerup="handlePointerEnd"
          @pointercancel="handlePointerEnd"
          @lostpointercapture="handlePointerEnd"
          @auxclick.prevent
        >
          <img
            v-if="imageSource && !showError"
            ref="imageElement"
            class="reference-window__image"
            :style="{ transform: imageTransform, opacity: imageOpacity }"
            :src="imageSource"
            :alt="asset?.filename ?? ''"
            decoding="async"
            @load="handleImageLoad"
            @error="imageFailed = true"
          >
          <p v-else-if="isLoading" class="reference-window__state">{{ $t("viewer.loading") }}</p>
          <p v-else class="reference-window__state reference-window__state--error">{{ errorMessage }}</p>
        </section>
      </template>
    </AppContextMenu>

    <div class="reference-window__toolbar iv-glass-surface" role="toolbar" :aria-label="t('viewer.reference')">
      <AppTooltip :content="t(mirrored ? 'viewer.unmirror' : 'viewer.mirror')">
        <template #trigger>
          <button
            class="reference-window__toolbar-button"
            :class="{ 'is-active': mirrored }"
            type="button"
            :aria-label="t(mirrored ? 'viewer.unmirror' : 'viewer.mirror')"
            :aria-pressed="mirrored"
            @click="toggleMirror"
          >
            <FlipHorizontal :size="16" :stroke-width="1.8" aria-hidden="true" />
          </button>
        </template>
      </AppTooltip>

      <div class="reference-window__tool-wrap">
        <AppTooltip :content="t('viewer.opacity', { value: opacity })">
          <template #trigger>
            <button
              class="reference-window__toolbar-button"
              type="button"
              :aria-label="t('viewer.opacity', { value: opacity })"
              :aria-expanded="opacityPopoverOpen"
              @click="toggleOpacityPopover"
            >
              <Droplets :size="16" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <label v-if="opacityPopoverOpen" class="reference-window__opacity-control iv-glass-surface">
          <span>{{ t("viewer.opacity", { value: opacity }) }}</span>
          <input
            type="range"
            min="20"
            max="100"
            step="1"
            :value="opacity"
            :aria-label="t('viewer.opacity', { value: opacity })"
            @input="setOpacityFromInput"
            @change="saveReferenceState"
          >
        </label>
      </div>

      <AppTooltip :content="t('viewer.alwaysOnTop')">
        <template #trigger>
          <button
            class="reference-window__toolbar-button"
            :class="{ 'is-active': alwaysOnTop }"
            type="button"
            :aria-label="t('viewer.alwaysOnTop')"
            :aria-pressed="alwaysOnTop"
            :disabled="isTopmostUpdating"
            @click="toggleAlwaysOnTop"
          >
            <Pin :size="16" :stroke-width="1.8" aria-hidden="true" />
          </button>
        </template>
      </AppTooltip>

      <div class="reference-window__tool-wrap">
        <AppTooltip :content="t('viewer.more')">
          <template #trigger>
            <button
              class="reference-window__toolbar-button"
              type="button"
              :aria-label="t('viewer.more')"
              :aria-expanded="moreMenuOpen"
              @click="toggleMoreMenu"
            >
              <Ellipsis :size="16" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <div v-if="moreMenuOpen" class="reference-window__more-menu iv-glass-surface" role="menu">
          <button type="button" role="menuitem" @click="resetFit(); moreMenuOpen = false">
            {{ t("viewer.fit") }}
          </button>
          <button type="button" role="menuitem" @click="setActualSize(); moreMenuOpen = false">
            {{ t("viewer.actualSize") }}
          </button>
          <button type="button" role="menuitem" :disabled="isLockUpdating" @click="toggleLock">
            <Lock v-if="!locked" :size="16" :stroke-width="1.8" aria-hidden="true" />
            <Unlock v-else :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ t(locked ? "viewer.unlock" : "viewer.lock") }}</span>
          </button>
        </div>
      </div>

      <AppTooltip :content="t('window.close')">
        <template #trigger>
          <button
            class="reference-window__toolbar-button"
            type="button"
            :aria-label="t('window.close')"
            @click="closeWindow"
          >
            <X :size="16" :stroke-width="1.8" aria-hidden="true" />
          </button>
        </template>
      </AppTooltip>
    </div>
  </main>
</template>

<style scoped lang="scss">
.reference-window {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: var(--iv-image-critical-surface);
}

.reference-window__surface {
  display: grid;
  position: absolute;
  inset: 0;
  place-items: center;
  min-width: 0;
  min-height: 0;
  touch-action: none;
  cursor: move;
}

.reference-window__surface.is-zoomed {
  cursor: grab;
}

.reference-window__surface.is-panning {
  cursor: grabbing;
}

.reference-window__image {
  position: relative;
  z-index: 0;
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  pointer-events: none;
  user-select: none;
  transform-origin: center;
  transition: transform var(--iv-motion-fast) ease-out, opacity var(--iv-motion-fast) ease;
  will-change: transform;
  -webkit-user-drag: none;
}

.reference-window__state {
  margin: 0;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.reference-window__state--error {
  color: var(--iv-text-disabled);
}

.reference-window__toolbar {
  position: absolute;
  z-index: 1;
  top: var(--iv-space-2);
  right: var(--iv-space-2);
  display: flex;
  align-items: flex-start;
  gap: var(--iv-space-1);
  padding: var(--iv-control-inset);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-md);
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--iv-motion-fast) ease;
}

.reference-window:hover .reference-window__toolbar,
.reference-window:focus-within .reference-window__toolbar {
  opacity: 1;
  pointer-events: auto;
}

.reference-window__tool-wrap {
  position: relative;
}

.reference-window__toolbar-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  padding: 0;
  color: var(--iv-text-primary);
  cursor: pointer;
  background-color: transparent;
  border: 1px solid transparent;
  border-radius: var(--iv-radius-sm);
  transition: background-color var(--iv-motion-fast) ease, opacity var(--iv-motion-fast) ease;
}

.reference-window__toolbar-button:hover:not(:disabled),
.reference-window__more-menu button:hover {
  background-color: var(--iv-overlay-surface-hover);
}

.reference-window__toolbar-button.is-active {
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-overlay-accent-active);
}

.reference-window__toolbar-button:focus-visible,
.reference-window__more-menu button:focus-visible,
.reference-window__opacity-control input:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.reference-window__toolbar-button:disabled {
  cursor: default;
  opacity: 0.56;
}

.reference-window__opacity-control,
.reference-window__more-menu {
  position: absolute;
  top: calc(100% + var(--iv-space-2));
  right: 0;
  z-index: 1;
  color: var(--iv-text-primary);
  background-color: var(--iv-glass-bg-strong);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  box-shadow: var(--iv-shadow-floating);
}

.reference-window__opacity-control {
  display: grid;
  gap: var(--iv-space-2);
  width: 176px;
  padding: var(--iv-space-3);
  color: var(--iv-overlay-text-status);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.reference-window__opacity-control input {
  width: 100%;
  accent-color: var(--iv-accent);
}

.reference-window__more-menu {
  display: grid;
  gap: var(--iv-space-1);
  min-width: 120px;
  padding: var(--iv-space-2);
}

.reference-window__more-menu button {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  color: var(--iv-text-primary);
  font: inherit;
  font-size: var(--iv-font-size-13);
  text-align: left;
  cursor: pointer;
  background: transparent;
  border: 0;
  border-radius: var(--iv-radius-xs);
}

.reference-window__more-menu button:disabled {
  cursor: default;
  opacity: 0.56;
}

@media (prefers-reduced-motion: reduce) {
  .reference-window__image,
  .reference-window__toolbar,
  .reference-window__toolbar-button {
    transition: none;
  }
}
</style>
