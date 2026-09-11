<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ArrowLeft, RefreshCw, Sparkles } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import GalleryGrid from "../gallery/components/GalleryGrid.vue";
import QuickPreviewOverlay from "../gallery/components/QuickPreviewOverlay.vue";
import { useQuickPreviewStore } from "../gallery/quickPreviewStore";
import { useFavoriteStore } from "../favorites/favoriteStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset } from "../library/types";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import type { SmartCollectionRule } from "./smartCollectionApi";
import { useSmartCollectionStore } from "./smartCollectionStore";

const route = useRoute();
const router = useRouter();
const smartCollectionStore = useSmartCollectionStore();
const thumbnailStore = useThumbnailStore();
const favoriteStore = useFavoriteStore();
const quickPreviewStore = useQuickPreviewStore();
const viewerStore = useViewerStore();
const images = ref<ImageAsset[]>([]);
const galleryGrid = ref<InstanceType<typeof GalleryGrid> | null>(null);
const selectedImageIds = ref<string[]>([]);
const activeImageId = ref<string | null>(null);
const isLoading = ref(true);
const isMissing = ref(false);
let activeThumbnailWindowScope: string | null = null;
const collectionId = computed(() =>
  typeof route.params.collectionId === "string" ? route.params.collectionId : "",
);
const collection = computed(() =>
  smartCollectionStore.collections.find((item) => item.id === collectionId.value) ?? null,
);
const previewAsset = computed(() =>
  images.value.find((asset) => asset.id === quickPreviewStore.currentImageId) ?? null,
);

watch(
  collectionId,
  async (id) => {
    const thumbnailWindowScope = id ? `smart-collection:${id}` : null;
    if (activeThumbnailWindowScope && activeThumbnailWindowScope !== thumbnailWindowScope) {
      await thumbnailStore.releaseThumbnailWindow(activeThumbnailWindowScope);
    }
    activeThumbnailWindowScope = thumbnailWindowScope;
    if (thumbnailWindowScope) {
      await thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
    }
    const viewerContext = viewerStore.consumeReturnContext("smart-collection", id);
    isLoading.value = true;
    isMissing.value = false;
    images.value = [];
    quickPreviewStore.close();
    await Promise.all([smartCollectionStore.load(), favoriteStore.load()]);
    if (!smartCollectionStore.collections.some((item) => item.id === id)) {
      isMissing.value = true;
      isLoading.value = false;
      return;
    }
    images.value = await smartCollectionStore.loadImages(id);
    isLoading.value = false;
    if (viewerContext) {
      const availableIds = new Set(images.value.map((asset) => asset.id));
      selectedImageIds.value = viewerContext.selectedImageIds.filter((imageId) => availableIds.has(imageId));
      activeImageId.value = availableIds.has(viewerContext.activeImageId)
        ? viewerContext.activeImageId
        : selectedImageIds.value[0] ?? null;
      await nextTick();
      galleryGrid.value?.restoreScrollTop(viewerContext.scrollTop);
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  quickPreviewStore.close();
  if (activeThumbnailWindowScope) void thumbnailStore.releaseThumbnailWindow(activeThumbnailWindowScope);
});

function ruleLabel(rule: SmartCollectionRule) {
  if (rule.kind === "format") return "smartCollections.rules.pngImages";
  if (rule.kind === "landscape") return "smartCollections.rules.landscape";
  return "smartCollections.rules.largeImages";
}

async function refresh() {
  if (!collectionId.value) return;
  images.value = await smartCollectionStore.loadImages(collectionId.value);
}

function requestThumbnailWindow(window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }) {
  if (!activeThumbnailWindowScope) return;
  void thumbnailStore.syncThumbnailWindow(activeThumbnailWindowScope, window.viewport, window.prefetch);
}

function openPreview(asset: ImageAsset) {
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;
  void quickPreviewStore.open(asset);
}

async function openFullViewer(asset: ImageAsset) {
  const resultImageIds = images.value.map((item) => item.id);
  if (!collectionId.value || !resultImageIds.includes(asset.id)) return;
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;

  viewerStore.open({
    source: "smart-collection",
    entryLibraryId: asset.libraryId,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(images.value),
    activeImageId: asset.id,
    returnPath: `/smart-collections/${collectionId.value}`,
    sourceId: collectionId.value,
    selectedImageIds: selectedImageIds.value,
    scrollTop: galleryGrid.value?.getScrollTop() ?? 0,
    layoutMode: "grid",
    thumbnailSize: null,
  });
  quickPreviewStore.close();
  await router.push({ name: "viewer", params: { libraryId: asset.libraryId } });
}

function toggleFavorite(asset: ImageAsset) {
  void favoriteStore.toggle(asset.id);
}

function navigatePreview(direction: -1 | 1) {
  const currentIndex = images.value.findIndex((asset) => asset.id === quickPreviewStore.currentImageId);
  const nextAsset = images.value[currentIndex + direction];
  if (nextAsset) openPreview(nextAsset);
}
</script>

<template>
  <div class="smart-collection-detail-page">
    <div class="smart-collection-detail-page__inner">
      <header class="smart-collection-detail-page__header">
        <div>
          <RouterLink class="smart-collection-detail-page__back" :to="{ name: 'smart-collections' }">
            <ArrowLeft :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("smartCollections.detailBack") }}</span>
          </RouterLink>
          <h1>{{ collection?.name ?? $t("smartCollections.title") }}</h1>
          <p v-if="collection">{{ $t(ruleLabel(collection.rule)) }} · {{ $t("smartCollections.count", { count: images.length }) }}</p>
        </div>
        <AppButton variant="secondary" :disabled="isLoading" @click="refresh">
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("smartCollections.refresh") }}</span>
        </AppButton>
      </header>

      <p v-if="smartCollectionStore.error && !isMissing" class="smart-collection-detail-page__alert" role="alert">
        {{ $t(`smartCollections.errors.${smartCollectionStore.error}`) }}
      </p>

      <section v-if="isLoading" class="smart-collection-detail-page__state" aria-busy="true">
        <RefreshCw class="smart-collection-detail-page__spinner" :size="24" :stroke-width="1.8" aria-hidden="true" />
        <p>{{ $t("smartCollections.loadingImages") }}</p>
      </section>

      <section v-else-if="isMissing" class="smart-collection-detail-page__state">
        <h2>{{ $t("smartCollections.missingTitle") }}</h2>
        <p>{{ $t("smartCollections.missingDescription") }}</p>
        <RouterLink :to="{ name: 'smart-collections' }">{{ $t("smartCollections.detailBack") }}</RouterLink>
      </section>

      <section v-else-if="images.length === 0" class="smart-collection-detail-page__state">
        <Sparkles :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("smartCollections.emptyResultTitle") }}</h2>
        <p>{{ $t("smartCollections.emptyResultDescription") }}</p>
      </section>

      <GalleryGrid
        v-else
        ref="galleryGrid"
        :assets="images"
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
.smart-collection-detail-page { height: 100%; min-height: 100%; padding: var(--iv-content-padding); }
.smart-collection-detail-page__inner { display: grid; grid-template-rows: auto minmax(0, 1fr); gap: var(--iv-space-6); width: 100%; height: 100%; min-height: 0; max-width: var(--iv-content-max-width); margin: 0 auto; }
.smart-collection-detail-page__header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--iv-space-4); }
.smart-collection-detail-page__back { display: inline-flex; align-items: center; gap: var(--iv-space-2); color: var(--iv-text-secondary); font-size: var(--iv-font-size-13); line-height: var(--iv-line-height-18); }
.smart-collection-detail-page__back:hover, .smart-collection-detail-page__state a { color: var(--iv-accent-hover); }
.smart-collection-detail-page__header h1, .smart-collection-detail-page__header p, .smart-collection-detail-page__state h2, .smart-collection-detail-page__state p { margin: 0; }
.smart-collection-detail-page__header h1 { margin-top: var(--iv-space-2); font-size: var(--iv-font-size-24); font-weight: var(--iv-font-weight-semibold); line-height: var(--iv-line-height-32); }
.smart-collection-detail-page__header p, .smart-collection-detail-page__state p { margin-top: var(--iv-space-1); color: var(--iv-text-secondary); font-size: var(--iv-font-size-14); line-height: var(--iv-line-height-20); }
.smart-collection-detail-page__alert { grid-column: 1 / -1; margin: 0; padding: var(--iv-space-3) var(--iv-space-4); color: var(--iv-danger); background-color: var(--iv-danger-soft); border: 1px solid var(--iv-danger); border-radius: var(--iv-radius-sm); }
.smart-collection-detail-page__state { display: grid; place-content: center; justify-items: center; gap: var(--iv-space-3); min-height: 260px; padding: var(--iv-space-8); color: var(--iv-text-secondary); text-align: center; background-color: var(--iv-surface); border: 1px solid var(--iv-border-subtle); border-radius: var(--iv-radius-md); }
.smart-collection-detail-page__state h2 { color: var(--iv-text-primary); font-size: var(--iv-font-size-16); font-weight: var(--iv-font-weight-semibold); line-height: var(--iv-line-height-22); }
.smart-collection-detail-page__spinner { animation: smart-collection-detail-spin 900ms linear infinite; }
@keyframes smart-collection-detail-spin { to { transform: rotate(360deg); } }
@media (max-width: 640px) { .smart-collection-detail-page__header { flex-direction: column; } }
@media (prefers-reduced-motion: reduce) { .smart-collection-detail-page__spinner { animation: none; } }
</style>
