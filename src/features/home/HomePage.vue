<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { Clock3, FolderOpen, Library } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import { useLibraryStore } from "../library/libraryStore";
import { useRecentImageStore } from "../library/recentImageStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset, Library as LibraryRecord } from "../library/types";
import { deriveGalleryAssets } from "../gallery/browseAssets";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import ThumbnailPreview from "../library/components/ThumbnailPreview.vue";
import HomeSection from "./components/HomeSection.vue";
import LibraryCard from "./components/LibraryCard.vue";
const { locale, t } = useI18n();
const router = useRouter();
const libraryStore = useLibraryStore();
const recentImageStore = useRecentImageStore();
const thumbnailStore = useThumbnailStore();
const viewerStore = useViewerStore();
const recentLibraries = computed(() => libraryStore.libraries.slice(0, 4));
const knownImageCount = computed(() =>
  libraryStore.libraries.reduce((total, library) => total + (library.imageCount ?? 0), 0),
);
const unavailableLibraryCount = computed(() =>
  libraryStore.libraries.filter((library) => library.imageCount === null).length,
);
const headerStats = computed(() => {
  const unavailable = unavailableLibraryCount.value > 0
    ? t("home.unavailableCount", { count: unavailableLibraryCount.value })
    : "";

  return t("home.stats", {
    libraries: t("home.libraryCount", libraryStore.libraries.length),
    images: t("home.imageCount", knownImageCount.value),
    unavailable,
  });
});
const recentImages = computed(() =>
  recentImageStore.entries
    .map((entry) =>
      libraryStore.assetsByLibrary[entry.libraryId]?.find((asset) => asset.id === entry.imageId),
    )
    .filter((asset): asset is ImageAsset => Boolean(asset)),
);
const thumbnailCandidates = computed(() => {
  const uniqueAssets = new Map<string, ImageAsset>();

  for (const asset of recentImages.value) {
    uniqueAssets.set(asset.id, asset);
  }
  for (const library of recentLibraries.value) {
    const coverAsset = libraryStore.assetsByLibrary[library.id]?.[0];
    if (coverAsset) uniqueAssets.set(coverAsset.id, coverAsset);
  }

  return [...uniqueAssets.values()].slice(0, 12);
});
const libraryError = computed(() =>
  libraryStore.error ? t(`library.errors.${libraryStore.error}`) : null,
);
const dateFormatter = computed(
  () => new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }),
);

onMounted(() => {
  viewerStore.consumeReturnContext("home");
  void libraryStore.loadLibraries();
});

watch(
  [thumbnailCandidates, () => thumbnailStore.queueStatus?.cancelled],
  ([assets]) => {
    void thumbnailStore.ensureThumbnails(assets);
  },
  { immediate: true },
);

function formatDate(timestamp: number) {
  return dateFormatter.value.format(timestamp);
}

function imageCountLabel(library: LibraryRecord) {
  if (libraryStore.scannerStates[library.id]?.status === "scanning") {
    return t("library.scanning");
  }

  return library.imageCount === null
    ? t("library.imageCountUnknown")
    : t("home.imageCount", library.imageCount);
}

function coverAsset(library: LibraryRecord) {
  return libraryStore.assetsByLibrary[library.id]?.[0];
}

function openLibrary(library: LibraryRecord) {
  void router.push({ name: "gallery", params: { libraryId: library.id } });
}

function openRecentImage(asset: ImageAsset) {
  const galleryAssets = deriveGalleryAssets(
    libraryStore.assetsByLibrary[asset.libraryId] ?? [],
    "name",
    "all",
    locale.value,
  );
  const resultImageIds = galleryAssets.map((item) => item.id);
  if (!resultImageIds.includes(asset.id)) return;

  recentImageStore.recordViewed(asset);
  viewerStore.open({
    source: "home",
    entryLibraryId: asset.libraryId,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(galleryAssets),
    activeImageId: asset.id,
    returnPath: "/",
    sourceId: null,
    selectedImageIds: [asset.id],
    scrollTop: 0,
    sort: "name",
    fileType: "all",
    layoutMode: "grid",
    thumbnailSize: null,
  });
  void router.push({ name: "viewer", params: { libraryId: asset.libraryId } });
}
</script>

<template>
  <div class="home-page">
    <div class="home-page__inner">
      <header class="home-page__header">
        <div>
          <h1>{{ $t("home.title") }}</h1>
          <p>{{ headerStats }}</p>
        </div>
        <AppButton variant="secondary" :disabled="libraryStore.isAdding" @click="libraryStore.addLibrary">
          <FolderOpen :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ libraryStore.isAdding ? $t("library.adding") : $t("home.openFolder") }}</span>
        </AppButton>
      </header>

      <HomeSection :title="$t('home.recentLibraries')" :icon="Library" :show-view-all="false">
        <p v-if="libraryStore.isLoading" class="home-library-state" aria-busy="true">
          {{ $t("library.loading") }}
        </p>
        <p v-else-if="libraryError" class="home-library-state home-library-state--error" role="alert">
          {{ libraryError }}
        </p>
        <div v-else-if="recentLibraries.length === 0" class="home-library-empty">
          <div>
            <h3>{{ $t("library.emptyTitle") }}</h3>
            <p>{{ $t("library.emptyDescription") }}</p>
          </div>
          <AppButton :disabled="libraryStore.isAdding" @click="libraryStore.addLibrary">
            <FolderOpen :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t("home.openFolder") }}</span>
          </AppButton>
        </div>
        <div v-else class="library-grid">
          <LibraryCard
            v-for="library in recentLibraries"
            :key="library.id"
            :library="library"
            :created-label="$t('library.added', { date: formatDate(library.createdAt) })"
            :image-count-label="imageCountLabel(library)"
            :thumbnail-record="coverAsset(library) ? thumbnailStore.recordsByImageId[coverAsset(library)?.id] : null"
            :thumbnail-label="coverAsset(library)?.filename ?? library.name"
            :generating-label="$t('home.thumbnailGenerating')"
            :unavailable-label="$t('home.thumbnailUnavailable')"
            @open="openLibrary"
          />
        </div>
      </HomeSection>

      <HomeSection :title="$t('home.recentImages')" :icon="Clock3" :show-view-all="false">
        <div v-if="recentImages.length > 0" class="image-grid image-grid--recent">
          <button
            v-for="asset in recentImages"
            :key="asset.id"
            class="image-tile"
            type="button"
            :aria-label="asset.filename"
            @click="openRecentImage(asset)"
          >
            <ThumbnailPreview
              :record="thumbnailStore.recordsByImageId[asset.id] ?? null"
              :label="asset.filename"
              :generating-label="$t('home.thumbnailGenerating')"
              :unavailable-label="$t('home.thumbnailUnavailable')"
            />
          </button>
        </div>
        <p v-else class="home-image-empty">{{ $t('home.noRecentImages') }}</p>
      </HomeSection>

    </div>
  </div>
</template>

<style scoped lang="scss">
.home-page {
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.home-page__inner {
  display: grid;
  gap: var(--iv-space-6);
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.home-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-6);
}

.home-page__header h1 {
  margin: 0;
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.home-page__header p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.library-grid,
.image-grid {
  display: grid;
  min-width: 0;
}

.library-grid {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--iv-space-3);
}

.home-library-state,
.home-library-empty {
  margin: 0;
  color: var(--iv-text-secondary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.home-library-state {
  padding: var(--iv-space-4);
}

.home-library-state--error {
  color: var(--iv-danger);
}

.home-library-empty {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-4);
  min-height: 96px;
  padding: var(--iv-space-4);
}

.home-library-empty h3,
.home-library-empty p {
  margin: 0;
}

.home-image-empty {
  margin: 0;
  padding: var(--iv-space-4);
  color: var(--iv-text-secondary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.home-library-empty p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.image-grid--recent {
  grid-template-columns: repeat(auto-fill, minmax(var(--iv-home-recent-preview-width), 144px));
  grid-auto-rows: var(--iv-home-recent-preview-height);
  justify-content: start;
  gap: var(--iv-space-3);
}

.image-tile {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  height: var(--iv-home-recent-preview-height);
  min-width: 0;
  padding: 0;
  overflow: hidden;
  color: var(--iv-text-secondary);
  text-align: left;
  background-color: var(--iv-surface-hover);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
  cursor: pointer;
  transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    transform var(--iv-motion-fast) ease;
}

.image-tile:hover {
  background-color: var(--iv-surface);
  border-color: var(--iv-accent);
}

.image-tile:active {
  transform: scale(var(--iv-press-scale));
}

.image-tile:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.image-tile :deep(.thumbnail-preview),
.image-tile :deep(.thumbnail-placeholder) {
  width: 100%;
  height: 100%;
}

@media (max-width: 920px) {
  .library-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 680px) {
  .home-page__header {
    flex-direction: column;
  }

  .library-grid,
  .image-grid--recent {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: var(--iv-home-recent-preview-height);
  }

  .home-library-empty {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
