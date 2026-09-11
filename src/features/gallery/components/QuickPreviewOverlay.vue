<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type CSSProperties } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ChevronLeft, ChevronRight, Maximize2, X } from "lucide-vue-next";
import { useI18n } from "vue-i18n";
import AppTooltip from "../../../shared/components/ui/AppTooltip.vue";
import { clampImagePan } from "../../../shared/imageViewport";
import { isPointerInteractiveTarget } from "../../../shared/utils/isPointerInteractiveTarget";
import type { ImageAsset } from "../../library/types";
import { useQuickPreviewStore } from "../quickPreviewStore";

const props = defineProps<{
  asset: ImageAsset;
  sourcePath: string | null;
  isLoading: boolean;
  hasError: boolean;
  errorCode?: string | null;
  closeLabel: string;
  openFullLabel: string;
  previousLabel: string;
  nextLabel: string;
  loadingLabel: string;
  unavailableLabel: string;
}>();

const { t } = useI18n();
const quickPreviewStore = useQuickPreviewStore();
const QUICK_PREVIEW_MAX_SCALE_MULTIPLIER = 4;

const emit = defineEmits<{
  close: [];
  previous: [];
  next: [];
  "open-full": [];
}>();

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const imageFailed = ref(false);
const controlsVisible = ref(true);
const surface = ref<HTMLElement | null>(null);
const imageElement = ref<HTMLImageElement | null>(null);
const fitScale = ref<number | null>(null);
const isDragging = ref(false);
let resizeObserver: ResizeObserver | null = null;
let controlsTimer: ReturnType<typeof setTimeout> | null = null;
let dragState: {
  pointerId: number;
  startX: number;
  startY: number;
  translateX: number;
  translateY: number;
} | null = null;

const imageSource = computed(() => {
  if (!props.sourcePath) return null;
  return isTauriEnvironment ? convertFileSrc(props.sourcePath) : props.sourcePath;
});
const showError = computed(() => props.hasError || imageFailed.value);
const unavailableMessage = computed(() => {
  if (imageFailed.value) return t("viewer.sourceUnreadable");
  switch (props.errorCode) {
    case "source_missing": return t("viewer.sourceMissing");
    case "source_unreadable":
    case "decode_failed": return t("viewer.sourceUnreadable");
    case "unsupported_format": return t("viewer.unsupportedFormat");
    case "library_unavailable":
    case "library_not_found": return t("viewer.libraryUnavailable");
    default: return props.unavailableLabel;
  }
});
const maximumScale = computed(() => Math.max(
  fitScale.value ?? 1,
  (fitScale.value ?? 1) * QUICK_PREVIEW_MAX_SCALE_MULTIPLIER,
));
const imageTransform = computed(() =>
  `translate3d(${quickPreviewStore.translateX}px, ${quickPreviewStore.translateY}px, 0) scale(${quickPreviewStore.scale})`,
);
const imageStyle = computed<CSSProperties>(() => {
  const image = imageElement.value;
  if (!image?.naturalWidth || !image.naturalHeight || fitScale.value === null) {
    return {
      maxWidth: "100%",
      maxHeight: "100%",
      objectFit: "contain",
      transform: imageTransform.value,
    };
  }

  return {
    width: `${image.naturalWidth}px`,
    height: `${image.naturalHeight}px`,
    transform: imageTransform.value,
  };
});

watch(imageSource, () => {
  imageFailed.value = false;
  fitScale.value = null;
  quickPreviewStore.resetTransform();
});

watch(() => props.asset.id, () => {
  imageFailed.value = false;
  fitScale.value = null;
  quickPreviewStore.resetTransform();
});

function revealControls() {
  controlsVisible.value = true;
  if (controlsTimer !== null) window.clearTimeout(controlsTimer);
  controlsTimer = window.setTimeout(() => {
    controlsVisible.value = false;
    controlsTimer = null;
  }, 1800);
}

onBeforeUnmount(() => {
  if (controlsTimer !== null) window.clearTimeout(controlsTimer);
  resizeObserver?.disconnect();
  dragState = null;
  quickPreviewStore.resetTransform();
});

onMounted(() => {
  revealControls();
  resizeObserver = new ResizeObserver(updateFitScale);
  if (surface.value) resizeObserver.observe(surface.value);
  void nextTick(updateFitScale);
});

function updateFitScale() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight) return;

  const nextFitScale = Math.min(
    surface.value.clientWidth / image.naturalWidth,
    surface.value.clientHeight / image.naturalHeight,
  );
  const previousFitScale = fitScale.value;
  const wasAtFit = previousFitScale === null
    || Math.abs(quickPreviewStore.scale - previousFitScale) < 0.01;
  fitScale.value = nextFitScale;

  const nextMaximumScale = nextFitScale * QUICK_PREVIEW_MAX_SCALE_MULTIPLIER;
  if (wasAtFit || quickPreviewStore.scale < nextFitScale) {
    quickPreviewStore.resetTransform(nextFitScale);
  } else if (quickPreviewStore.scale > nextMaximumScale) {
    quickPreviewStore.setTransform({
      scale: nextMaximumScale,
      translateX: quickPreviewStore.translateX,
      translateY: quickPreviewStore.translateY,
    });
    clampPan();
  } else {
    clampPan();
  }
}

function clampPan() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight || fitScale.value === null) return;

  const clamped = clampImagePan({
    panX: quickPreviewStore.translateX,
    panY: quickPreviewStore.translateY,
    surfaceWidth: surface.value.clientWidth,
    surfaceHeight: surface.value.clientHeight,
    imageWidth: image.naturalWidth * quickPreviewStore.scale,
    imageHeight: image.naturalHeight * quickPreviewStore.scale,
  });
  quickPreviewStore.setTransform({
    scale: quickPreviewStore.scale,
    translateX: clamped.panX,
    translateY: clamped.panY,
  });
}

function handleImageLoad() {
  updateFitScale();
}

function handleWheel(event: WheelEvent) {
  if (!surface.value || event.target !== imageElement.value
    || !imageElement.value?.naturalWidth || event.deltaY === 0 || fitScale.value === null) return;

  event.preventDefault();
  const delta = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * 16 : event.deltaY;
  const nextScale = Math.max(
    fitScale.value,
    Math.min(maximumScale.value, quickPreviewStore.scale * Math.exp(-delta * 0.0015)),
  );
  if (Math.abs(nextScale - quickPreviewStore.scale) < 0.001) return;

  const bounds = surface.value.getBoundingClientRect();
  const cursorX = event.clientX - bounds.left - bounds.width / 2;
  const cursorY = event.clientY - bounds.top - bounds.height / 2;
  const ratio = nextScale / quickPreviewStore.scale;
  quickPreviewStore.setTransform({
    scale: nextScale,
    translateX: quickPreviewStore.translateX * ratio + cursorX * (1 - ratio),
    translateY: quickPreviewStore.translateY * ratio + cursorY * (1 - ratio),
  });
  clampPan();
  revealControls();
}

function handlePointerDown(event: PointerEvent) {
  if (!surface.value || event.target !== imageElement.value || isPointerInteractiveTarget(event.target)) return;
  const canPanWithLeftButton = event.button === 0 && fitScale.value !== null
    && quickPreviewStore.scale > fitScale.value + 0.01;
  if (event.button !== 1 && !canPanWithLeftButton) return;

  event.preventDefault();
  dragState = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    translateX: quickPreviewStore.translateX,
    translateY: quickPreviewStore.translateY,
  };
  isDragging.value = true;
  surface.value.setPointerCapture(event.pointerId);
}

function handlePointerMove(event: PointerEvent) {
  if (!dragState || event.pointerId !== dragState.pointerId) return;

  quickPreviewStore.setTransform({
    scale: quickPreviewStore.scale,
    translateX: dragState.translateX + event.clientX - dragState.startX,
    translateY: dragState.translateY + event.clientY - dragState.startY,
  });
  clampPan();
}

function handlePointerEnd(event: PointerEvent) {
  if (!dragState || event.pointerId !== dragState.pointerId) return;

  if (surface.value?.hasPointerCapture(event.pointerId)) surface.value.releasePointerCapture(event.pointerId);
  dragState = null;
  isDragging.value = false;
  clampPan();
}

function resetFit() {
  if (fitScale.value === null) return;
  quickPreviewStore.resetTransform(fitScale.value);
}
</script>

<template>
  <Teleport to="body">
    <Transition name="quick-preview">
      <section class="quick-preview" role="dialog" aria-modal="true" :aria-label="asset.filename" tabindex="-1"
        @mousemove="revealControls" @wheel="handleWheel" @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove" @pointerup="handlePointerEnd" @pointercancel="handlePointerEnd" :class="{
          'is-dragging': isDragging,
          'is-pan-enabled': fitScale !== null && quickPreviewStore.scale > fitScale + 0.01,
        }">
        <div class="quick-preview__surface">
          <div ref="surface" class="quick-preview__viewport">
            <img v-if="imageSource && !showError" class="quick-preview__image" :src="imageSource" :alt="asset.filename"
              ref="imageElement" :style="imageStyle" draggable="false" decoding="async" @error="imageFailed = true"
              @load="handleImageLoad" @dblclick="resetFit">
            <p v-else-if="isLoading && !showError" class="quick-preview__state">
              {{ loadingLabel }}
            </p>
            <p v-else class="quick-preview__state quick-preview__state--error">
              {{ unavailableMessage }}
            </p>
          </div>
        </div>

        <header v-show="controlsVisible" class="quick-preview__topbar">
          <p :title="asset.filename">{{ asset.filename }}</p>
          <div class="quick-preview__actions">
            <AppTooltip :content="openFullLabel">
              <template #trigger>
                <button class="iv-glass-surface" type="button" :aria-label="openFullLabel" @click="emit('open-full')">
                  <Maximize2 :size="18" :stroke-width="1.8" aria-hidden="true" />
                </button>
              </template>
            </AppTooltip>
            <AppTooltip :content="closeLabel">
              <template #trigger>
                <button class="iv-glass-surface" type="button" :aria-label="closeLabel" @click="emit('close')">
                  <X :size="18" :stroke-width="1.8" aria-hidden="true" />
                </button>
              </template>
            </AppTooltip>
          </div>
        </header>

        <div v-show="controlsVisible" class="quick-preview__navigation" aria-hidden="false">
          <AppTooltip :content="previousLabel" side="right">
            <template #trigger>
              <button class="iv-glass-surface" type="button" :aria-label="previousLabel" @click="emit('previous')">
                <ChevronLeft :size="22" :stroke-width="1.8" aria-hidden="true" />
              </button>
            </template>
          </AppTooltip>
          <AppTooltip :content="nextLabel" side="left">
            <template #trigger>
              <button class="iv-glass-surface" type="button" :aria-label="nextLabel" @click="emit('next')">
                <ChevronRight :size="22" :stroke-width="1.8" aria-hidden="true" />
              </button>
            </template>
          </AppTooltip>
        </div>
      </section>
    </Transition>
  </Teleport>
</template>

<style scoped lang="scss">
.quick-preview {
  position: fixed;
  inset: 0;
  z-index: var(--iv-layer-modal);
  display: grid;
  place-items: center;
  padding: 0;
  background-color: var(--iv-preview-scrim);
}

.quick-preview.is-pan-enabled {
  .quick-preview__viewport {
    cursor: grab;
  }
}

.quick-preview.is-dragging {
  .quick-preview__viewport {
    cursor: grabbing;
  }
}

.quick-preview__surface {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background-color: var(--iv-viewer-matte-auto-surface);
}

.quick-preview__viewport {
  position: absolute;
  inset: calc(var(--iv-space-5) + var(--iv-control-height) + var(--iv-space-3)) var(--iv-space-6) var(--iv-space-6);
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  touch-action: none;
}

.quick-preview__image {
  display: block;
  max-width: none;
  max-height: none;
  object-fit: contain;
  transform-origin: center;
  user-select: none;
  -webkit-user-drag: none;
}

.quick-preview__state {
  margin: 0;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.quick-preview__state--error {
  color: var(--iv-text-disabled);
}

.quick-preview__topbar {
  position: absolute;
  inset: var(--iv-space-5) var(--iv-space-6) auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-4);
  color: var(--iv-text-primary);
  pointer-events: none;
}

.quick-preview__topbar p {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.quick-preview__actions,
.quick-preview__navigation {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  pointer-events: auto;
}

.quick-preview button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-height);
  height: var(--iv-control-height);
  padding: 0;
  color: var(--iv-text-primary);
  background-color: var(--iv-overlay-surface);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: background-color var(--iv-motion-fast) ease, opacity var(--iv-motion-fast) ease;
}

.quick-preview button:hover {
  background-color: var(--iv-overlay-surface-hover);
}

.quick-preview button:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.quick-preview__navigation {
  position: absolute;
  inset: 50% var(--iv-space-6) auto;
  justify-content: space-between;
  pointer-events: none;
  transform: translateY(-50%);
}

.quick-preview__navigation :deep(button) {
  pointer-events: auto;
}

.quick-preview-enter-active,
.quick-preview-leave-active {
  transition: opacity var(--iv-motion-normal) ease;
}

.quick-preview-enter-active .quick-preview__surface,
.quick-preview-leave-active .quick-preview__surface {
  transition: transform var(--iv-motion-normal) ease;
}

.quick-preview-enter-from,
.quick-preview-leave-to {
  opacity: 0;
}

.quick-preview-enter-from .quick-preview__surface,
.quick-preview-leave-to .quick-preview__surface {
  transform: scale(0.99);
}
</style>
