<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { ArrowLeft, FolderOpen, FolderPlus, RefreshCw, Tags } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppSelect, { type AppSelectOption } from "../../shared/components/ui/AppSelect.vue";
import { useLibraryStore } from "../library/libraryStore";
import { useRecentImageStore } from "../library/recentImageStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset } from "../library/types";
import { deriveGalleryAssets, type GalleryFileType, type GallerySort } from "./browseAssets";
import GalleryGrid from "./components/GalleryGrid.vue";
import QuickPreviewOverlay from "./components/QuickPreviewOverlay.vue";
import { useQuickPreviewStore } from "./quickPreviewStore";
import { useGallerySelectionStore } from "./selectionStore";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import { openReferenceWindow } from "../reference/referenceWindowApi";
import { useFavoriteStore } from "../favorites/favoriteStore";
import AddToCollectionDialog from "../collections/AddToCollectionDialog.vue";
import TagAssignmentDialog from "../tags/TagAssignmentDialog.vue";
import { useTagStore } from "../tags/tagStore";
import { getDuplicateIndicators, type DuplicateMatchKind } from "../fingerprint/fingerprintApi";
import { shortcutDispatcher } from "../../shared/shortcuts/shortcutDispatcher";
import { clampThumbnailSize } from "./virtualGrid";

const { locale, t } = useI18n();
const route = useRoute();
const router = useRouter();
const libraryStore = useLibraryStore();
const recentImageStore = useRecentImageStore();
const thumbnailStore = useThumbnailStore();
const selectionStore = useGallerySelectionStore();
const quickPreviewStore = useQuickPreviewStore();
const viewerStore = useViewerStore();
const favoriteStore = useFavoriteStore();
const tagStore = useTagStore();
const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const isPreparing = ref(true);
const sort = ref<GallerySort>("name");
const fileType = ref<GalleryFileType>("all");
const thumbnailSize = ref(160);
const libraryId = computed(() => (typeof route.params.libraryId === "string" ? route.params.libraryId : ""));
const library = computed(() => libraryStore.libraries.find((item) => item.id === libraryId.value) ?? null);
const assets = computed(() => libraryStore.assetsByLibrary[libraryId.value] ?? []);
const galleryAssets = computed(() => deriveGalleryAssets(assets.value, sort.value, fileType.value, locale.value));
const sortOptions = computed<AppSelectOption[]>(() => [
  { value: "name", label: t("gallery.sortName") },
  { value: "createdAt", label: t("gallery.sortCreatedAt") },
  { value: "modifiedAt", label: t("gallery.sortModifiedAt") },
]);
const fileTypeOptions = computed<AppSelectOption[]>(() => [
  { value: "all", label: t("gallery.fileTypeAll") },
  ...[...new Set(assets.value.map((asset) => asset.extension))]
    .sort()
    .map((extension) => ({ value: extension, label: extension.toUpperCase() })),
]);
const scannerState = computed(() => libraryStore.scannerStates[libraryId.value]);
const scannerError = computed(() => scannerState.value?.status === "error" ? scannerState.value.error : null);
const isScanning = computed(() => scannerState.value?.status === "scanning");
const galleryGrid = ref<InstanceType<typeof GalleryGrid> | null>(null);
const collectionDialogImageIds = ref<string[]>([]);
const tagDialogImageIds = ref<string[]>([]);
const previewAsset = computed(() =>
  galleryAssets.value.find((asset) => asset.id === quickPreviewStore.currentImageId) ?? null,
);
const selectedAssets = computed(() => {
  const selectedIds = new Set(selectionStore.selectedIds);
  return galleryAssets.value.filter((asset) => selectedIds.has(asset.id));
});
const duplicateIndicators = ref<Record<string, DuplicateMatchKind | null>>({});
const knownDuplicateIndicatorIds = new Set<string>();
const pendingDuplicateIndicatorIds = new Set<string>();
let duplicateIndicatorRequest = 0;

watch(
  libraryId,
  async (id) => {
    duplicateIndicatorRequest += 1;
    duplicateIndicators.value = {};
    knownDuplicateIndicatorIds.clear();
    pendingDuplicateIndicatorIds.clear();
    const viewerContext = viewerStore.consumeReturnContext("library", id);
    selectionStore.clear();
    quickPreviewStore.close();
    if (viewerContext) {
      if (viewerContext.sort) sort.value = viewerContext.sort;
      if (viewerContext.fileType) fileType.value = viewerContext.fileType as GalleryFileType;
      if (viewerContext.thumbnailSize !== undefined && viewerContext.thumbnailSize !== null) {
        thumbnailSize.value = clampThumbnailSize(viewerContext.thumbnailSize);
      }
    }
    isPreparing.value = true;
    await thumbnailStore.syncThumbnailWindow(id, [], []);
    await Promise.all([libraryStore.ensureLibraryAssets(id), favoriteStore.load()]);
    isPreparing.value = false;

    if (viewerContext) {
      selectionStore.restore(viewerContext.selectedImageIds, viewerContext.activeImageId);
      selectionStore.retain(assetIds(assets.value));
      await nextTick();
      galleryGrid.value?.restoreScrollTop(viewerContext.scrollTop);
    }
  },
  { immediate: true },
);

watch(assets, (items) => selectionStore.retain(assetIds(items)), { immediate: true });

watch(fileTypeOptions, (options) => {
  if (!options.some((option) => option.value === fileType.value)) fileType.value = "all";
});

let unregisterShortcuts: (() => void) | null = null;

onMounted(() => {
  unregisterShortcuts = shortcutDispatcher.register("gallery", {
    "gallery.preview": () => {
      if (quickPreviewStore.isOpen) quickPreviewStore.close();
      else openSelectedPreview();
    },
    "gallery.viewer": () => {
      if (quickPreviewStore.isOpen) {
        if (previewAsset.value) openFullViewer(previewAsset.value);
        return;
      }
      const activeAsset = galleryAssets.value.find((asset) => asset.id === selectionStore.activeId);
      if (activeAsset) openFullViewer(activeAsset);
      else return false;
    },
    "gallery.reference": () => {
      if (quickPreviewStore.isOpen) return false;
      const activeAsset = galleryAssets.value.find((asset) => asset.id === selectionStore.activeId);
      if (activeAsset) openReference(activeAsset);
      else return false;
    },
    "gallery.previewPrevious": () => {
      if (!quickPreviewStore.isOpen) return false;
      void navigatePreview(-1);
    },
    "gallery.previewNext": () => {
      if (!quickPreviewStore.isOpen) return false;
      void navigatePreview(1);
    },
    "gallery.closePreview": () => {
      if (!quickPreviewStore.isOpen) return false;
      quickPreviewStore.close();
    },
    "gallery.selectAll": () => {
      if (quickPreviewStore.isOpen) return false;
      selectionStore.selectedIds = galleryAssets.value.map((asset) => asset.id);
      selectionStore.activeId = selectionStore.selectedIds[0] ?? null;
    },
  });
});

onBeforeUnmount(() => {
  unregisterShortcuts?.();
  selectionStore.clear();
  quickPreviewStore.close();
  void thumbnailStore.releaseThumbnailWindow(libraryId.value);
});

function retry() {
  void libraryStore.refreshLibraryAssets(libraryId.value);
}

function requestThumbnailWindow(window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }) {
  void thumbnailStore.syncThumbnailWindow(libraryId.value, window.viewport, window.prefetch);
  void tagStore.loadImageTagsForImages([...window.viewport, ...window.prefetch].map((asset) => asset.id));
  void loadDuplicateIndicators([...window.viewport, ...window.prefetch]);
}

async function loadDuplicateIndicators(windowAssets: ImageAsset[]) {
  if (!isTauriEnvironment) return;

  const imageIds = [...new Set(windowAssets.map((asset) => asset.id))]
    .filter((imageId) => !knownDuplicateIndicatorIds.has(imageId) && !pendingDuplicateIndicatorIds.has(imageId));
  if (imageIds.length === 0) return;

  const request = duplicateIndicatorRequest;
  for (let start = 0; start < imageIds.length; start += 128) {
    const batch = imageIds.slice(start, start + 128);
    for (const imageId of batch) pendingDuplicateIndicatorIds.add(imageId);
    try {
      const indicators = await getDuplicateIndicators(batch);
      if (request !== duplicateIndicatorRequest) return;

      const next = { ...duplicateIndicators.value };
      for (const imageId of batch) {
        knownDuplicateIndicatorIds.add(imageId);
        next[imageId] = null;
      }
      for (const indicator of indicators) next[indicator.imageId] = indicator.matchKind;
      duplicateIndicators.value = next;
    } catch {
      // Retry only when this virtual window is requested again.
    } finally {
      for (const imageId of batch) pendingDuplicateIndicatorIds.delete(imageId);
    }
  }
}

function selectAsset(asset: (typeof assets.value)[number], event: MouseEvent) {
  selectionStore.select(asset.id, {
    toggle: event.ctrlKey || event.metaKey,
    range: event.shiftKey,
  }, event.shiftKey ? galleryAssets.value.map((item) => item.id) : undefined);
}

function openSelectedPreview() {
  const activeAsset = galleryAssets.value.find((asset) => asset.id === selectionStore.activeId);
  if (activeAsset) void quickPreviewStore.open(activeAsset);
}

function openReference(asset: ImageAsset) {
  void openReferenceWindow(asset).catch(() => undefined);
}

function toggleFavorite(asset: ImageAsset) {
  void favoriteStore.toggle(asset.id);
}

function openCollectionDialog() {
  collectionDialogImageIds.value = selectedAssets.value.map((asset) => asset.id);
}

function openTagDialog() {
  tagDialogImageIds.value = selectedAssets.value.map((asset) => asset.id);
}

function openFullViewer(asset: ImageAsset) {
  const resultImageIds = galleryAssets.value.map((item) => item.id);
  if (resultImageIds.length === 0) return;

  recentImageStore.recordViewed(asset);
  viewerStore.open({
    source: "library",
    entryLibraryId: libraryId.value,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(galleryAssets.value),
    activeImageId: asset.id,
    returnPath: `/library/${libraryId.value}`,
    sourceId: libraryId.value,
    selectedImageIds: [...selectionStore.selectedIds],
    scrollTop: galleryGrid.value?.getScrollTop() ?? 0,
    sort: sort.value,
    fileType: fileType.value,
    layoutMode: "grid",
    thumbnailSize: thumbnailSize.value,
  });
  quickPreviewStore.close();
  void router.push({ name: "viewer", params: { libraryId: libraryId.value } });
}

async function navigatePreview(direction: -1 | 1) {
  const currentIndex = galleryAssets.value.findIndex(
    (asset) => asset.id === quickPreviewStore.currentImageId,
  );
  const nextAsset = galleryAssets.value[currentIndex + direction];
  if (!nextAsset) return;

  selectionStore.select(nextAsset.id, { toggle: false, range: false });
  await quickPreviewStore.open(nextAsset);
  await nextTick();
  galleryGrid.value?.scrollToAsset(nextAsset.id);
}

function* assetIds(items: ImageAsset[]) {
  for (const asset of items) yield asset.id;
}
</script>

<template>
  <div class="gallery-page">
    <div class="gallery-page__inner">
      <header class="gallery-page__header">
        <div>
          <RouterLink class="gallery-page__back" :to="{ name: 'library' }">
            <ArrowLeft :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("gallery.back") }}</span>
          </RouterLink>
          <h1>{{ library?.name ?? $t("gallery.title") }}</h1>
          <p v-if="library">{{ $t("gallery.filteredItems", { shown: galleryAssets.length, total: assets.length }) }}</p>
          <div v-if="library" class="gallery-page__controls">
            <span class="gallery-page__scope">
              <FolderOpen :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t("gallery.currentLibrary", { name: library.name }) }}</span>
            </span>
            <div class="gallery-page__control">
              <span>{{ $t("gallery.sortLabel") }}</span>
              <AppSelect v-model="sort" :options="sortOptions" :aria-label="$t('gallery.sortLabel')" />
            </div>
            <div class="gallery-page__control">
              <span>{{ $t("gallery.fileTypeLabel") }}</span>
              <AppSelect v-model="fileType" :options="fileTypeOptions" :aria-label="$t('gallery.fileTypeLabel')" />
            </div>
          </div>
        </div>
        <div v-if="library && !isPreparing && !isScanning" class="gallery-page__header-actions">
          <AppButton variant="secondary" :disabled="selectedAssets.length === 0" @click="openTagDialog">
            <Tags :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("gallery.manageTags") }}</span>
          </AppButton>
          <AppButton variant="secondary" :disabled="selectedAssets.length === 0" @click="openCollectionDialog">
            <FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("gallery.addToCollection") }}</span>
          </AppButton>
          <AppButton variant="secondary" @click="retry">
            <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("gallery.retry") }}</span>
          </AppButton>
        </div>
      </header>

      <section v-if="isPreparing || isScanning" class="gallery-state" aria-busy="true">
        <RefreshCw class="gallery-state__spinner" :size="24" :stroke-width="1.8" aria-hidden="true" />
        <p>{{ isScanning ? $t("gallery.scanning") : $t("gallery.loading") }}</p>
      </section>

      <section v-else-if="!library" class="gallery-state">
        <FolderOpen :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("gallery.missingTitle") }}</h2>
        <p>{{ $t("gallery.missingDescription") }}</p>
        <RouterLink class="gallery-state__link" :to="{ name: 'library' }">
          {{ $t("gallery.back") }}
        </RouterLink>
      </section>

      <section v-else-if="scannerError" class="gallery-state gallery-state--error" role="alert">
        <h2>{{ $t(`library.scannerErrors.${scannerError}`) }}</h2>
        <AppButton variant="secondary" @click="retry">
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("gallery.retry") }}</span>
        </AppButton>
      </section>

      <section v-else-if="assets.length === 0" class="gallery-state">
        <FolderOpen :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("gallery.emptyTitle") }}</h2>
        <p>{{ $t("gallery.emptyDescription") }}</p>
      </section>

      <section v-else-if="galleryAssets.length === 0" class="gallery-state">
        <FolderOpen :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("gallery.noMatchesTitle") }}</h2>
        <p>{{ $t("gallery.noMatchesDescription") }}</p>
      </section>

      <GalleryGrid
        v-else
        ref="galleryGrid"
        :assets="galleryAssets"
        :records-by-image-id="thumbnailStore.recordsByImageId"
        :thumbnail-size="thumbnailSize"
        :enable-thumbnail-resize="true"
        :generating-label="$t('home.thumbnailGenerating')"
        :unavailable-label="$t('home.thumbnailUnavailable')"
        :selected-ids="selectionStore.selectedIds"
        :active-id="selectionStore.activeId"
        :favorite-ids="favoriteStore.favoriteIds"
        :pending-favorite-ids="favoriteStore.pendingIds"
        :favorite-label="$t('gallery.favorite')"
        :unfavorite-label="$t('gallery.unfavorite')"
        :duplicate-kinds="duplicateIndicators"
        :tags-by-image-id="tagStore.imageTagsByImageId"
        :exact-match-label="$t('duplicates.exactMatch')"
        :similar-label="$t('duplicates.similar')"
        @thumbnail-window-change="requestThumbnailWindow"
        @thumbnail-size-change="thumbnailSize = $event"
        @select="selectAsset"
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
        @open-full="previewAsset && openFullViewer(previewAsset)"
      />

      <AddToCollectionDialog
        :open="collectionDialogImageIds.length > 0"
        :image-ids="collectionDialogImageIds"
        @update:open="collectionDialogImageIds = $event ? collectionDialogImageIds : []"
      />
      <TagAssignmentDialog
        :open="tagDialogImageIds.length > 0"
        :image-ids="tagDialogImageIds"
        @update:open="tagDialogImageIds = $event ? tagDialogImageIds : []"
      />
    </div>
  </div>
</template>

<style scoped lang="scss">
.gallery-page {
  height: 100%;
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.gallery-page__inner {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: var(--iv-space-6);
  width: 100%;
  height: 100%;
  min-height: 0;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.gallery-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-4);
}

.gallery-page__header-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--iv-space-2);
}

.gallery-page__back,
.gallery-state__link {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.gallery-page__back:hover,
.gallery-state__link:hover {
  color: var(--iv-text-primary);
}

.gallery-page__header h1,
.gallery-page__header p,
.gallery-state h2,
.gallery-state p {
  margin: 0;
}

.gallery-page__header h1 {
  margin-top: var(--iv-space-2);
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.gallery-page__header p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.gallery-page__controls {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: var(--iv-space-3);
  margin-top: var(--iv-space-4);
}

.gallery-page__scope {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-width: 0;
  height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  overflow: hidden;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  text-overflow: ellipsis;
  white-space: nowrap;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
}

.gallery-page__scope span {
  overflow: hidden;
  text-overflow: ellipsis;
}

.gallery-page__control {
  display: grid;
  gap: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.gallery-page__control :deep(.app-select) {
  width: 160px;
}

.gallery-state {
  display: grid;
  justify-items: center;
  gap: var(--iv-space-3);
  min-height: 260px;
  padding: var(--iv-space-8);
  color: var(--iv-text-secondary);
  text-align: center;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.gallery-state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  line-height: var(--iv-line-height-22);
}

.gallery-state p {
  max-width: 360px;
}

.gallery-state--error h2 {
  color: var(--iv-danger);
}

.gallery-state__spinner {
  animation: gallery-spin 900ms linear infinite;
}

@keyframes gallery-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 680px) {
  .gallery-page__header {
    flex-direction: column;
  }

  .gallery-page__controls {
    align-items: stretch;
  }
}

@media (prefers-reduced-motion: reduce) {
  .gallery-state__spinner {
    animation: none;
  }
}
</style>
