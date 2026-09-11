<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { Heart, RefreshCw, Search, SlidersHorizontal, Tags, X } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppSelect, { type AppSelectOption } from "../../shared/components/ui/AppSelect.vue";
import GalleryGrid from "../gallery/components/GalleryGrid.vue";
import QuickPreviewOverlay from "../gallery/components/QuickPreviewOverlay.vue";
import { useQuickPreviewStore } from "../gallery/quickPreviewStore";
import { useCollectionStore } from "../collections/collectionStore";
import { useFavoriteStore } from "../favorites/favoriteStore";
import { useLibraryStore } from "../library/libraryStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset } from "../library/types";
import { useSmartCollectionStore } from "../smartCollections/smartCollectionStore";
import { useTagStore } from "../tags/tagStore";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import { useSearchStore } from "./searchStore";

const router = useRouter();
const { t } = useI18n();
const searchStore = useSearchStore();
const libraryStore = useLibraryStore();
const collectionStore = useCollectionStore();
const smartCollectionStore = useSmartCollectionStore();
const tagStore = useTagStore();
const favoriteStore = useFavoriteStore();
const thumbnailStore = useThumbnailStore();
const quickPreviewStore = useQuickPreviewStore();
const viewerStore = useViewerStore();
const galleryGrid = ref<InstanceType<typeof GalleryGrid> | null>(null);
const thumbnailWindowScope = "search";
const selectedImageIds = ref<string[]>([]);
const activeImageId = ref<string | null>(null);
const previewAsset = computed(() =>
  searchStore.assets.find((asset) => asset.id === quickPreviewStore.currentImageId) ?? null,
);
const libraryOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allLibraries") },
  ...libraryStore.libraries.map((library) => ({ value: library.id, label: library.name })),
]);
const collectionOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allCollections") },
  ...collectionStore.collections.map((collection) => ({ value: collection.id, label: collection.name })),
]);
const formatOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allFormats") },
  ...["png", "jpg", "webp", "gif"].map((format) => ({ value: format, label: format.toUpperCase() })),
]);
const sizeOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allSizes") },
  { value: "1920", label: t("search.minimumWidth1920") },
  { value: "3840", label: t("search.minimumWidth3840") },
]);
const orientationOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allOrientations") },
  { value: "landscape", label: t("search.orientationLandscape") },
  { value: "portrait", label: t("search.orientationPortrait") },
  { value: "square", label: t("search.orientationSquare") },
]);
const smartCollectionOptions = computed<AppSelectOption[]>(() => [
  { value: "", label: t("search.allSmartCollections") },
  ...smartCollectionStore.collections.map((collection) => ({ value: collection.id, label: collection.name })),
]);
const selectedLibraryId = computed({
  get: () => searchStore.filters.libraryId ?? "",
  set: (value: string) => {
    searchStore.filters.libraryId = value || null;
    void refresh();
  },
});
const selectedCollectionId = computed({
  get: () => searchStore.filters.collectionId ?? "",
  set: (value: string) => {
    searchStore.filters.collectionId = value || null;
    void refresh();
  },
});
const selectedFormat = computed({
  get: () => searchStore.filters.formats[0] ?? "",
  set: (value: string) => {
    searchStore.setFormat(value || null);
    void refresh();
  },
});
const selectedMinimumWidth = computed({
  get: () => String(searchStore.filters.metadata.minimumWidth ?? ""),
  set: (value: string) => {
    searchStore.setMinimumWidth(value ? Number(value) : null);
    void refresh();
  },
});
const selectedOrientation = computed({
  get: () => searchStore.filters.metadata.orientation ?? "",
  set: (value: string) => {
    searchStore.setOrientation(value ? value as "landscape" | "portrait" | "square" : null);
    void refresh();
  },
});
const selectedSmartCollectionId = computed({
  get: () => searchStore.filters.smartCollectionId ?? "",
  set: (value: string) => {
    searchStore.filters.smartCollectionId = value || null;
    void refresh();
  },
});
const selectedTagIds = computed(() => new Set(searchStore.filters.tagIds));

watch(
  () => tagStore.tags.map((tag) => tag.id),
  (tagIds) => {
    const validIds = new Set(tagIds);
    const nextIds = searchStore.filters.tagIds.filter((id) => validIds.has(id));
    if (nextIds.length !== searchStore.filters.tagIds.length) {
      searchStore.filters.tagIds = nextIds;
      if (searchStore.hasSearched) void refresh();
    }
  },
);

watch(
  () => tagStore.revision,
  () => {
    if (searchStore.hasSearched) void refresh();
  },
);

onMounted(async () => {
  await Promise.all([
    libraryStore.loadLibraries(),
    collectionStore.load(),
    smartCollectionStore.load(),
    tagStore.load(),
    favoriteStore.load(),
  ]);
  if (searchStore.hasCriteria) await refresh();

  const viewerContext = viewerStore.consumeReturnContext("search");
  if (!viewerContext?.search) return;

  searchStore.restoreViewContext(viewerContext.search);
  const validTagIds = new Set(tagStore.tags.map((tag) => tag.id));
  searchStore.filters.tagIds = searchStore.filters.tagIds.filter((id) => validTagIds.has(id));
  await refresh();
  const availableIds = new Set(searchStore.assets.map((asset) => asset.id));
  selectedImageIds.value = viewerContext.selectedImageIds.filter((id) => availableIds.has(id));
  activeImageId.value = availableIds.has(viewerContext.activeImageId)
    ? viewerContext.activeImageId
    : selectedImageIds.value[0] ?? null;
  await nextTick();
  galleryGrid.value?.restoreScrollTop(viewerContext.scrollTop);
});

onBeforeUnmount(() => {
  void thumbnailStore.releaseThumbnailWindow(thumbnailWindowScope);
});

async function refresh() {
  void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
  const assets = await searchStore.search();
  if (assets.length === 0) void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
}

function toggleFavorites() {
  searchStore.filters.favoritesOnly = !searchStore.filters.favoritesOnly;
  void refresh();
}

function toggleTag(tagId: string) {
  searchStore.filters.tagIds = selectedTagIds.value.has(tagId)
    ? searchStore.filters.tagIds.filter((id) => id !== tagId)
    : [...searchStore.filters.tagIds, tagId];
  void refresh();
}

function clearSearch() {
  searchStore.clear();
  quickPreviewStore.close();
  void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
}

function requestThumbnailWindow(window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }) {
  void thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, window.viewport, window.prefetch);
  void tagStore.loadImageTagsForImages([...window.viewport, ...window.prefetch].map((asset) => asset.id));
}

function openPreview(asset: ImageAsset) {
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;
  void quickPreviewStore.open(asset);
}

async function openFullViewer(asset: ImageAsset) {
  const resultImageIds = searchStore.assets.map((item) => item.id);
  if (!resultImageIds.includes(asset.id)) return;
  
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;

  viewerStore.open({
    source: "search",
    entryLibraryId: asset.libraryId,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(searchStore.assets),
    activeImageId: asset.id,
    returnPath: "/search",
    sourceId: null,
    selectedImageIds: selectedImageIds.value,
    scrollTop: galleryGrid.value?.getScrollTop() ?? 0,
    layoutMode: "grid",
    thumbnailSize: null,
    search: searchStore.snapshotViewContext(),
  });
  quickPreviewStore.close();
  await router.push({ name: "viewer", params: { libraryId: asset.libraryId } });
}

async function toggleFavorite(asset: ImageAsset) {
  await favoriteStore.toggle(asset.id);
  if (searchStore.filters.favoritesOnly) await refresh();
}

function navigatePreview(direction: -1 | 1) {
  const currentIndex = searchStore.assets.findIndex((asset) => asset.id === quickPreviewStore.currentImageId);
  const nextAsset = searchStore.assets[currentIndex + direction];
  if (nextAsset) openPreview(nextAsset);
}
</script>

<template>
  <div class="search-page">
    <div class="search-page__inner">
      <header class="search-page__header">
        <div>
          <h1>{{ $t('search.title') }}</h1>
          <p v-if="searchStore.hasSearched">{{ $t('search.resultCount', { count: searchStore.assets.length }) }}</p>
          <p v-else>{{ $t('search.subtitle') }}</p>
        </div>
        <div class="search-page__header-actions">
          <AppButton variant="secondary" :disabled="searchStore.isLoading" @click="refresh">
            <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t('search.refresh') }}</span>
          </AppButton>
          <AppButton variant="secondary" :disabled="!searchStore.hasCriteria" @click="clearSearch">
            <X :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t('search.clear') }}</span>
          </AppButton>
        </div>
      </header>

      <section class="search-filters" :aria-label="$t('search.filtersLabel')">
        <div class="search-filters__heading">
          <SlidersHorizontal :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t('search.filtersLabel') }}</span>
        </div>
        <AppSelect v-model="selectedLibraryId" :options="libraryOptions" :aria-label="$t('search.libraryFilter')" />
        <AppSelect v-model="selectedCollectionId" :options="collectionOptions" :aria-label="$t('search.collectionFilter')" />
        <AppSelect v-model="selectedFormat" :options="formatOptions" :aria-label="$t('search.formatFilter')" />
        <AppSelect v-model="selectedMinimumWidth" :options="sizeOptions" :aria-label="$t('search.sizeFilter')" />
        <AppSelect v-model="selectedOrientation" :options="orientationOptions" :aria-label="$t('search.orientationFilter')" />
        <AppSelect v-model="selectedSmartCollectionId" :options="smartCollectionOptions" :aria-label="$t('search.smartCollectionFilter')" />
        <button
          type="button"
          class="search-filter-chip"
          :class="{ 'is-selected': searchStore.filters.favoritesOnly }"
          :aria-pressed="searchStore.filters.favoritesOnly"
          @click="toggleFavorites"
        >
          <Heart :size="16" :stroke-width="1.8" :fill="searchStore.filters.favoritesOnly ? 'currentColor' : 'none'" aria-hidden="true" />
          <span>{{ $t('search.favoritesFilter') }}</span>
        </button>
        <div v-if="tagStore.tags.length > 0" class="search-filters__tags" :aria-label="$t('search.tagFilter')">
          <button
            v-for="tag in tagStore.tags"
            :key="tag.id"
            type="button"
            class="search-filter-chip"
            :class="{ 'is-selected': selectedTagIds.has(tag.id) }"
            :aria-pressed="selectedTagIds.has(tag.id)"
            @click="toggleTag(tag.id)"
          >
            <Tags :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ tag.name }}</span>
          </button>
        </div>
      </section>

      <section v-if="searchStore.isLoading" class="search-page__state" aria-busy="true">
        <Search :size="28" :stroke-width="1.6" aria-hidden="true" />
        <p>{{ $t('search.loading') }}</p>
      </section>
      <section v-else-if="searchStore.error" class="search-page__state search-page__state--error" role="alert">
        <h2>{{ $t('search.errorTitle') }}</h2>
        <p>{{ $t('search.errorDescription') }}</p>
      </section>
      <section v-else-if="!searchStore.hasSearched" class="search-page__state">
        <Search :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t('search.emptyTitle') }}</h2>
        <p>{{ $t('search.emptyDescription') }}</p>
      </section>
      <section v-else-if="searchStore.assets.length === 0" class="search-page__state">
        <Search :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t('search.noResultsTitle') }}</h2>
        <p>{{ $t('search.noResultsDescription') }}</p>
      </section>
      <GalleryGrid
        v-else
        ref="galleryGrid"
        :assets="searchStore.assets"
        :records-by-image-id="thumbnailStore.recordsByImageId"
        :generating-label="$t('home.thumbnailGenerating')"
        :unavailable-label="$t('home.thumbnailUnavailable')"
        :favorite-ids="favoriteStore.favoriteIds"
        :pending-favorite-ids="favoriteStore.pendingIds"
        :selected-ids="selectedImageIds"
        :active-id="activeImageId"
        :favorite-label="$t('gallery.favorite')"
        :unfavorite-label="$t('gallery.unfavorite')"
        :tags-by-image-id="tagStore.imageTagsByImageId"
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
.search-page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.search-page__inner {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.search-page__header,
.search-page__header-actions,
.search-filters,
.search-filters__tags {
  display: flex;
  align-items: center;
}

.search-page__header {
  justify-content: space-between;
  gap: var(--iv-space-4);
  padding-bottom: var(--iv-space-4);
}

.search-page__header-actions,
.search-filters,
.search-filters__tags {
  flex-wrap: wrap;
  gap: var(--iv-space-2);
}

.search-page__header h1,
.search-page__header p,
.search-page__state h2,
.search-page__state p {
  margin: 0;
}

.search-page__header h1 {
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.search-page__header p,
.search-page__state p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.search-filters {
  align-items: center;
  padding: var(--iv-space-3) 0 var(--iv-space-5);
  border-top: 1px solid var(--iv-border-subtle);
}

.search-filters__heading {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
}

.search-filters :deep(.app-select) {
  width: 164px;
}

.search-filter-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-secondary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-full);
  cursor: pointer;
}

.search-filter-chip:hover,
.search-filter-chip.is-selected {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent-soft);
  border-color: var(--iv-accent);
}

.search-filter-chip:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.search-page__state {
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

.search-page__state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  line-height: var(--iv-line-height-24);
}

.search-page__state--error h2 {
  color: var(--iv-danger);
}

@media (max-width: 680px) {
  .search-page__header {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
