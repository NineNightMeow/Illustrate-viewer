<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { Heart, RefreshCw } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import GalleryGrid from "../gallery/components/GalleryGrid.vue";
import QuickPreviewOverlay from "../gallery/components/QuickPreviewOverlay.vue";
import { useQuickPreviewStore } from "../gallery/quickPreviewStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset } from "../library/types";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import { useFavoriteStore } from "./favoriteStore";

const router = useRouter();
const favoriteStore = useFavoriteStore();
const thumbnailStore = useThumbnailStore();
const quickPreviewStore = useQuickPreviewStore();
const viewerStore = useViewerStore();
const favoriteAssets = ref<ImageAsset[]>([]);
const galleryGrid = ref<InstanceType<typeof GalleryGrid> | null>(null);
const selectedImageIds = ref<string[]>([]);
const activeImageId = ref<string | null>(null);
const thumbnailWindowScope = "favorites";
const previewAsset = computed(() =>
  favoriteAssets.value.find((asset) => asset.id === quickPreviewStore.currentImageId) ?? null,
);

onMounted(async () => {
  const viewerContext = viewerStore.consumeReturnContext("favorites");
  await thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
  await refresh();
  if (!viewerContext) return;

  const availableIds = new Set(favoriteAssets.value.map((asset) => asset.id));
  selectedImageIds.value = viewerContext.selectedImageIds.filter((id) => availableIds.has(id));
  activeImageId.value = availableIds.has(viewerContext.activeImageId)
    ? viewerContext.activeImageId
    : selectedImageIds.value[0] ?? null;
  await nextTick();
  galleryGrid.value?.restoreScrollTop(viewerContext.scrollTop);
});

async function refresh() {
  void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
  favoriteAssets.value = await favoriteStore.load();
}

function requestThumbnailWindow(window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }) {
  void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, window.viewport, window.prefetch);
}

onBeforeUnmount(() => {
  void thumbnailStore.releaseThumbnailWindow(thumbnailWindowScope);
});

function openPreview(asset: ImageAsset) {
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;
  void quickPreviewStore.open(asset);
}

async function toggleFavorite(asset: ImageAsset) {
  const remainsFavorite = await favoriteStore.toggle(asset.id);
  if (remainsFavorite) return;

  favoriteAssets.value = favoriteAssets.value.filter((item) => item.id !== asset.id);
  if (quickPreviewStore.currentImageId === asset.id) quickPreviewStore.close();
}

async function openFullViewer(asset: ImageAsset) {
  const resultImageIds = favoriteAssets.value.map((item) => item.id);
  if (!resultImageIds.includes(asset.id)) return;
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;

  viewerStore.open({
    source: "favorites",
    entryLibraryId: asset.libraryId,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(favoriteAssets.value),
    activeImageId: asset.id,
    returnPath: "/favorites",
    sourceId: null,
    selectedImageIds: selectedImageIds.value,
    scrollTop: galleryGrid.value?.getScrollTop() ?? 0,
    layoutMode: "grid",
    thumbnailSize: null,
  });
  quickPreviewStore.close();
  await router.push({ name: "viewer", params: { libraryId: asset.libraryId } });
}

function navigatePreview(direction: -1 | 1) {
  const currentIndex = favoriteAssets.value.findIndex(
    (asset) => asset.id === quickPreviewStore.currentImageId,
  );
  const nextAsset = favoriteAssets.value[currentIndex + direction];
  if (nextAsset) openPreview(nextAsset);
}
</script>

<template>
  <div class="favorites-page">
    <div class="favorites-page__inner">
      <header class="favorites-page__header">
        <div>
          <h1>{{ $t("favorites.title") }}</h1>
          <p>{{ $t("favorites.count", { count: favoriteAssets.length }) }}</p>
        </div>
        <AppButton variant="secondary" :disabled="favoriteStore.isLoading" @click="refresh">
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("favorites.refresh") }}</span>
        </AppButton>
      </header>

      <section v-if="favoriteStore.isLoading" class="favorites-state" aria-busy="true">
        <RefreshCw class="favorites-state__spinner" :size="24" :stroke-width="1.8" aria-hidden="true" />
        <p>{{ $t("favorites.loading") }}</p>
      </section>

      <section v-else-if="favoriteAssets.length === 0" class="favorites-state">
        <Heart :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("favorites.emptyTitle") }}</h2>
        <p>{{ $t("favorites.emptyDescription") }}</p>
      </section>

      <GalleryGrid
        v-else
        ref="galleryGrid"
        :assets="favoriteAssets"
        :records-by-image-id="thumbnailStore.recordsByImageId"
        :generating-label="$t('home.thumbnailGenerating')"
        :unavailable-label="$t('home.thumbnailUnavailable')"
        :favorite-ids="favoriteStore.favoriteIds"
        :pending-favorite-ids="favoriteStore.pendingIds"
        :selected-ids="selectedImageIds"
        :active-id="activeImageId"
        :favorite-label="$t('gallery.favorite')"
        :unfavorite-label="$t('gallery.unfavorite')"
        @thumbnail-window-change="requestThumbnailWindow"
        @select="openPreview"
        @open="openFullViewer"
        @toggle-favorite="toggleFavorite"
      />

      <QuickPreviewOverlay
        v-if="quickPreviewStore.isOpen && previewAsset"
        :asset="previewAsset"
        :source-path="quickPreviewStore.sourcePath"
        :is-loading="quickPreviewStore.isLoading"
        :has-error="quickPreviewStore.hasError"
        :error-code="quickPreviewStore.errorCode"
        :close-label="$t('gallery.quickPreviewClose')"
        :open-full-label="$t('gallery.quickPreviewOpenFull')"
        :previous-label="$t('gallery.quickPreviewPrevious')"
        :next-label="$t('gallery.quickPreviewNext')"
        :loading-label="$t('gallery.quickPreviewLoading')"
        :unavailable-label="$t('gallery.quickPreviewUnavailable')"
        @close="quickPreviewStore.close"
        @previous="navigatePreview(-1)"
        @next="navigatePreview(1)"
        @open-full="openFullViewer(previewAsset)"
      />
    </div>
  </div>
</template>

<style scoped lang="scss">
.favorites-page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.favorites-page__inner {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.favorites-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-6);
  padding-bottom: var(--iv-space-5);
}

.favorites-page__header h1,
.favorites-page__header p,
.favorites-state h2,
.favorites-state p {
  margin: 0;
}

.favorites-page__header h1 {
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.favorites-page__header p,
.favorites-state p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.favorites-state {
  display: grid;
  place-content: center;
  justify-items: center;
  flex: 1;
  gap: var(--iv-space-2);
  min-height: 240px;
  padding: var(--iv-space-6);
  color: var(--iv-text-secondary);
  text-align: center;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.favorites-state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-24);
}

.favorites-state__spinner {
  animation: favorites-spin 900ms linear infinite;
}

@keyframes favorites-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .favorites-state__spinner {
    animation: none;
  }
}
</style>
