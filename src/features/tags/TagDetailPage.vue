<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ArrowLeft, Minus, RefreshCw, Tags } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import GalleryGrid from "../gallery/components/GalleryGrid.vue";
import QuickPreviewOverlay from "../gallery/components/QuickPreviewOverlay.vue";
import { useQuickPreviewStore } from "../gallery/quickPreviewStore";
import { useFavoriteStore } from "../favorites/favoriteStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import type { ImageAsset } from "../library/types";
import { imageLibraryIdsFor, useViewerStore } from "../viewer/viewerStore";
import { listTagImages } from "./tagApi";
import { useTagStore } from "./tagStore";

const route = useRoute();
const router = useRouter();
const tagStore = useTagStore();
const favoriteStore = useFavoriteStore();
const thumbnailStore = useThumbnailStore();
const quickPreviewStore = useQuickPreviewStore();
const viewerStore = useViewerStore();
const assets = ref<ImageAsset[]>([]);
const galleryGrid = ref<InstanceType<typeof GalleryGrid> | null>(null);
const selectedImageIds = ref<string[]>([]);
const activeImageId = ref<string | null>(null);
const isLoading = ref(true);
let activeThumbnailWindowScope: string | null = null;
const tagId = computed(() => typeof route.params.tagId === "string" ? route.params.tagId : "");
const tag = computed(() => tagStore.tags.find((item) => item.id === tagId.value) ?? null);
const previewAsset = computed(() => assets.value.find((asset) => asset.id === quickPreviewStore.currentImageId) ?? null);

watch(tagId, () => void refresh(), { immediate: true });

onMounted(() => {
  void favoriteStore.load();
});

async function refresh() {
  const thumbnailWindowScope = tagId.value ? `tag:${tagId.value}` : null;
  if (activeThumbnailWindowScope && activeThumbnailWindowScope !== thumbnailWindowScope) {
    await thumbnailStore.releaseThumbnailWindow(activeThumbnailWindowScope);
  }
  activeThumbnailWindowScope = thumbnailWindowScope;
  if (thumbnailWindowScope) {
    await thumbnailStore.syncThumbnailWindow(thumbnailWindowScope, [], []);
  }
  const viewerContext = viewerStore.consumeReturnContext("tag", tagId.value);
  isLoading.value = true;
  await tagStore.load();
  if (!tagId.value || !tag.value) {
    assets.value = [];
    isLoading.value = false;
    return;
  }

  try {
    assets.value = await listTagImages(tagId.value);
    if (viewerContext) {
      const availableIds = new Set(assets.value.map((asset) => asset.id));
      selectedImageIds.value = viewerContext.selectedImageIds.filter((imageId) => availableIds.has(imageId));
      activeImageId.value = availableIds.has(viewerContext.activeImageId)
        ? viewerContext.activeImageId
        : selectedImageIds.value[0] ?? null;
      await nextTick();
      galleryGrid.value?.restoreScrollTop(viewerContext.scrollTop);
    }
  } catch {
    assets.value = [];
  } finally {
    isLoading.value = false;
  }
}

function requestThumbnailWindow(window: { viewport: ImageAsset[]; prefetch: ImageAsset[] }) {
  if (!activeThumbnailWindowScope) return;
  void thumbnailStore.syncThumbnailWindow(activeThumbnailWindowScope, window.viewport, window.prefetch);
  void tagStore.loadImageTagsForImages([...window.viewport, ...window.prefetch].map((asset) => asset.id));
}

onBeforeUnmount(() => {
  if (activeThumbnailWindowScope) void thumbnailStore.releaseThumbnailWindow(activeThumbnailWindowScope);
});

function openPreview(asset: ImageAsset) {
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;
  void quickPreviewStore.open(asset);
}

async function openFullViewer(asset: ImageAsset) {
  const resultImageIds = assets.value.map((item) => item.id);
  if (!tagId.value || !resultImageIds.includes(asset.id)) return;
  selectedImageIds.value = [asset.id];
  activeImageId.value = asset.id;

  viewerStore.open({
    source: "tag",
    entryLibraryId: asset.libraryId,
    resultImageIds,
    imageLibraryIds: imageLibraryIdsFor(assets.value),
    activeImageId: asset.id,
    returnPath: `/tags/${tagId.value}`,
    sourceId: tagId.value,
    selectedImageIds: selectedImageIds.value,
    scrollTop: galleryGrid.value?.getScrollTop() ?? 0,
    layoutMode: "grid",
    thumbnailSize: null,
  });
  quickPreviewStore.close();
  await router.push({ name: "viewer", params: { libraryId: asset.libraryId } });
}

async function toggleFavorite(asset: ImageAsset) {
  await favoriteStore.toggle(asset.id);
}

async function removeFromTag(asset: ImageAsset) {
  if (!tagId.value || !await tagStore.removeFromImages(tagId.value, [asset.id])) return;
  assets.value = assets.value.filter((item) => item.id !== asset.id);
  if (quickPreviewStore.currentImageId === asset.id) quickPreviewStore.close();
}

function navigatePreview(direction: -1 | 1) {
  const currentIndex = assets.value.findIndex((asset) => asset.id === quickPreviewStore.currentImageId);
  const nextAsset = assets.value[currentIndex + direction];
  if (nextAsset) openPreview(nextAsset);
}
</script>

<template>
  <div class="tag-detail-page">
    <div class="tag-detail-page__inner">
      <header class="tag-detail-page__header">
        <div>
          <RouterLink class="tag-detail-page__back" :to="{ name: 'tags' }">
            <ArrowLeft :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ $t('tags.detailBack') }}</span>
          </RouterLink>
          <h1>{{ tag?.name ?? $t('tags.title') }}</h1>
          <p v-if="tag">{{ $t('tags.imageCount', { count: assets.length }) }}</p>
        </div>
        <AppButton variant="secondary" :disabled="isLoading" @click="refresh">
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t('tags.refresh') }}</span>
        </AppButton>
      </header>

      <section v-if="isLoading" class="tag-detail-page__state" aria-busy="true">
        {{ $t('tags.loading') }}
      </section>
      <section v-else-if="!tag" class="tag-detail-page__state">
        <Tags :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t('tags.missingTitle') }}</h2>
        <p>{{ $t('tags.missingDescription') }}</p>
      </section>
      <section v-else-if="assets.length === 0" class="tag-detail-page__state">
        <Tags :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t('tags.emptyResultTitle') }}</h2>
        <p>{{ $t('tags.emptyResultDescription') }}</p>
      </section>
      <GalleryGrid
        v-else
        ref="galleryGrid"
        :assets="assets"
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
        :secondary-action-label="$t('tags.removeFromTag')"
        :secondary-action-icon="Minus"
        secondary-action-danger
        @thumbnail-window-change="requestThumbnailWindow"
        @select="openPreview"
        @open="openFullViewer"
        @toggle-favorite="toggleFavorite"
        @secondary-action="removeFromTag"
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
.tag-detail-page {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.tag-detail-page__inner {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.tag-detail-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-4);
  padding-bottom: var(--iv-space-5);
}

.tag-detail-page__back {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.tag-detail-page__back:hover {
  color: var(--iv-text-primary);
}

.tag-detail-page__header h1,
.tag-detail-page__header p,
.tag-detail-page__state h2,
.tag-detail-page__state p {
  margin: 0;
}

.tag-detail-page__header h1 {
  margin-top: var(--iv-space-2);
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.tag-detail-page__header p,
.tag-detail-page__state p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.tag-detail-page__state {
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

.tag-detail-page__state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  line-height: var(--iv-line-height-24);
}

@media (max-width: 680px) {
  .tag-detail-page__header {
    flex-direction: column;
  }
}
</style>
