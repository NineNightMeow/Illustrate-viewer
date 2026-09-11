<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Component } from "vue";
import GalleryItem from "./GalleryItem.vue";
import type { ImageAsset, ThumbnailRecord } from "../../library/types";
import { adjustThumbnailSize, createVirtualGridLayout, createVirtualGridRange, shouldResizeThumbnails } from "../virtualGrid";

const props = withDefaults(defineProps<{
  assets: ImageAsset[];
  thumbnailSize?: number;
  enableThumbnailResize?: boolean;
  recordsByImageId: Record<string, ThumbnailRecord>;
  generatingLabel: string;
  unavailableLabel: string;
  selectedIds?: string[];
  activeId?: string | null;
  favoriteIds?: string[];
  pendingFavoriteIds?: string[];
  favoriteLabel: string;
  unfavoriteLabel: string;
  duplicateKinds?: Record<string, "exact" | "similar" | null>;
  exactMatchLabel?: string;
  similarLabel?: string;
  secondaryActionLabel?: string;
  secondaryActionIcon?: Component | null;
  secondaryActionDanger?: boolean;
  tagsByImageId?: Record<string, { id: string; name: string }[]>;
}>(), {
  thumbnailSize: 160,
  enableThumbnailResize: false,
  selectedIds: () => [],
  activeId: null,
  favoriteIds: () => [],
  pendingFavoriteIds: () => [],
  duplicateKinds: () => ({}),
  exactMatchLabel: "",
  similarLabel: "",
  secondaryActionLabel: "",
  secondaryActionIcon: null,
  secondaryActionDanger: false,
  tagsByImageId: () => ({}),
});

const emit = defineEmits<{
  "thumbnail-window-change": [window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }];
  "thumbnail-size-change": [size: number];
  select: [asset: ImageAsset, event: MouseEvent];
  open: [asset: ImageAsset];
  "toggle-favorite": [asset: ImageAsset];
  "secondary-action": [asset: ImageAsset];
}>();

const viewport = ref<HTMLElement | null>(null);
const viewportWidth = ref(0);
const viewportHeight = ref(0);
const scrollTop = ref(0);
let resizeObserver: ResizeObserver | null = null;
let thumbnailWindowFrame: number | null = null;
let lastThumbnailWindowKey = "";

const layout = computed(() => createVirtualGridLayout(viewportWidth.value, props.assets.length, props.thumbnailSize));
const range = computed(() => createVirtualGridRange(layout.value, scrollTop.value, viewportHeight.value));
const viewportRange = computed(() =>
  createVirtualGridRange(layout.value, scrollTop.value, viewportHeight.value, 0),
);
const visibleAssets = computed(() =>
  props.assets.slice(range.value.startIndex, range.value.endIndex),
);
const selectedIds = computed(() => new Set(props.selectedIds));
const favoriteIds = computed(() => new Set(props.favoriteIds));
const pendingFavoriteIds = computed(() => new Set(props.pendingFavoriteIds));

function measureViewport() {
  if (!viewport.value) return;
  viewportWidth.value = viewport.value.clientWidth;
  viewportHeight.value = viewport.value.clientHeight;
}

function updateScroll() {
  scrollTop.value = viewport.value?.scrollTop ?? 0;
}

function handleWheel(event: WheelEvent) {
  if (!props.enableThumbnailResize || !shouldResizeThumbnails(event)) return;
  event.preventDefault();
  emit("thumbnail-size-change", adjustThumbnailSize(props.thumbnailSize, event.deltaY));
}

function scrollToAsset(imageId: string) {
  const assetIndex = props.assets.findIndex((asset) => asset.id === imageId);
  if (assetIndex < 0 || !viewport.value) return;

  const itemTop = Math.floor(assetIndex / layout.value.columns) * layout.value.rowHeight;
  const itemBottom = itemTop + layout.value.rowHeight;
  const viewportBottom = scrollTop.value + viewportHeight.value;
  if (itemTop >= scrollTop.value && itemBottom <= viewportBottom) return;

  viewport.value.scrollTo({
    top: itemTop < scrollTop.value ? itemTop : Math.max(0, itemBottom - viewportHeight.value),
    behavior: "auto",
  });
}

function getScrollTop() {
  return scrollTop.value;
}

function restoreScrollTop(top: number) {
  if (!viewport.value) return;

  viewport.value.scrollTop = Math.max(0, top);
  scrollTop.value = viewport.value.scrollTop;
  scheduleThumbnailWindow();
}

function scheduleThumbnailWindow() {
  if (thumbnailWindowFrame !== null) return;

  thumbnailWindowFrame = window.requestAnimationFrame(() => {
    thumbnailWindowFrame = null;
    const key = [
      viewportRange.value.startIndex,
      viewportRange.value.endIndex,
      range.value.startIndex,
      range.value.endIndex,
    ].join(":");
    if (key === lastThumbnailWindowKey) return;
    lastThumbnailWindowKey = key;

    emit("thumbnail-window-change", {
      viewport: props.assets.slice(viewportRange.value.startIndex, viewportRange.value.endIndex),
      prefetch: [
        ...props.assets.slice(range.value.startIndex, viewportRange.value.startIndex),
        ...props.assets.slice(viewportRange.value.endIndex, range.value.endIndex),
      ],
    });
  });
}

watch([viewportRange, range], scheduleThumbnailWindow, { immediate: true });

watch(
  () => props.assets,
  () => {
    lastThumbnailWindowKey = "";
    if (viewport.value) viewport.value.scrollTop = 0;
    scrollTop.value = 0;
    void nextTick(measureViewport);
    scheduleThumbnailWindow();
  },
);

onMounted(() => {
  measureViewport();
  resizeObserver = new ResizeObserver(measureViewport);
  if (viewport.value) resizeObserver.observe(viewport.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  if (thumbnailWindowFrame !== null) window.cancelAnimationFrame(thumbnailWindowFrame);
});

defineExpose({ getScrollTop, restoreScrollTop, scrollToAsset });
</script>

<template>
  <div ref="viewport" class="gallery-grid" role="list" @scroll.passive="updateScroll" @wheel="handleWheel">
    <div class="gallery-grid__spacer" :style="{ height: `${layout.totalHeight}px` }">
      <div
        class="gallery-grid__items"
        :style="{
          '--gallery-column-count': layout.columns,
          transform: `translateY(${range.offset}px)`,
        }"
      >
        <GalleryItem
          v-for="asset in visibleAssets"
          :key="asset.id"
          :asset="asset"
          :record="recordsByImageId[asset.id] ?? null"
          :generating-label="generatingLabel"
          :unavailable-label="unavailableLabel"
          :selected="selectedIds.has(asset.id)"
          :active="activeId === asset.id"
          :favorite="favoriteIds.has(asset.id)"
          :favorite-pending="pendingFavoriteIds.has(asset.id)"
          :favorite-label="favoriteLabel"
          :unfavorite-label="unfavoriteLabel"
          :duplicate-kind="duplicateKinds[asset.id] ?? null"
          :exact-match-label="exactMatchLabel"
          :similar-label="similarLabel"
          :secondary-action-label="secondaryActionLabel"
          :secondary-action-icon="secondaryActionIcon"
          :secondary-action-danger="secondaryActionDanger"
          :tags="tagsByImageId[asset.id] ?? []"
          @select="(selectedAsset, event) => emit('select', selectedAsset, event)"
          @open="(selectedAsset) => emit('open', selectedAsset)"
          @toggle-favorite="(selectedAsset) => emit('toggle-favorite', selectedAsset)"
          @secondary-action="(selectedAsset) => emit('secondary-action', selectedAsset)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.gallery-grid {
  min-height: 0;
  overflow: auto;
  padding: var(--iv-gallery-padding);
  overscroll-behavior: contain;
}

.gallery-grid__spacer {
  position: relative;
  min-height: 100%;
}

.gallery-grid__items {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-columns: repeat(var(--gallery-column-count), minmax(0, 1fr));
  gap: var(--iv-space-3);
  min-width: 0;
  will-change: transform;
}
</style>
