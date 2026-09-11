<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { ChevronRight, FolderOpen, FolderPlus, Image, RefreshCw, Trash2 } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import { formatPathForDisplay } from "../../shared/utils/formatPathForDisplay";
import ThumbnailPreview from "./components/ThumbnailPreview.vue";
import { useLibraryStore } from "./libraryStore";
import { useThumbnailStore } from "./thumbnailStore";
import type { ImageAsset, Library } from "./types";

const { locale, t } = useI18n();
const router = useRouter();
const libraryStore = useLibraryStore();
const thumbnailStore = useThumbnailStore();
const removalCandidate = ref<Library | null>(null);

const libraryError = computed(() =>
  libraryStore.error ? t("library.errors." + libraryStore.error) : null,
);
const dateFormatter = computed(
  () => new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }),
);
const coverAssets = computed(() =>
  libraryStore.libraries
    .map((library) => libraryStore.assetsByLibrary[library.id]?.[0])
    .filter((asset): asset is ImageAsset => Boolean(asset)),
);

onMounted(() => {
  void libraryStore.loadLibraries();
});

watch(
  coverAssets,
  (assets) => {
    void thumbnailStore.ensureThumbnails(assets);
  },
  { immediate: true },
);

function formatDate(timestamp: number) {
  return dateFormatter.value.format(timestamp);
}

function coverAsset(library: Library) {
  return libraryStore.assetsByLibrary[library.id]?.[0];
}

function imageCountLabel(library: Library) {
  const scanner = libraryStore.scannerStates[library.id];
  if (scanner?.status === "scanning") return t("library.scanning");
  if (scanner?.status === "error" && scanner.error) {
    return t("library.scannerErrors." + scanner.error);
  }

  return library.imageCount === null
    ? t("library.imageCountUnknown")
    : t("library.imagesCount", { count: library.imageCount });
}

function openLibrary(library: Library) {
  void router.push({ name: "gallery", params: { libraryId: library.id } });
}

function updateRemovalDialog(library: Library, open: boolean) {
  removalCandidate.value = open ? library : null;
}

async function removeLibrary(library: Library) {
  await libraryStore.removeLibrary(library.id);
  if (!libraryStore.error) removalCandidate.value = null;
}
</script>

<template>
  <div class="library-page">
    <div class="library-page__inner">
      <header class="library-page__header">
        <div>
          <h1>{{ $t("library.title") }}</h1>
          <p>{{ $t("library.subtitle") }}</p>
        </div>
        <AppButton :disabled="libraryStore.isAdding" @click="libraryStore.addLibrary">
          <FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ libraryStore.isAdding ? $t("library.adding") : $t("library.add") }}</span>
        </AppButton>
      </header>

      <p v-if="libraryError && libraryStore.libraries.length > 0" class="library-alert" role="alert">
        {{ libraryError }}
      </p>

      <section v-if="libraryStore.isLoading" class="library-state" aria-busy="true">
        <p>{{ $t("library.loading") }}</p>
      </section>

      <section v-else-if="libraryError && libraryStore.libraries.length === 0" class="library-state library-state--error" role="alert">
        <FolderOpen :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("library.title") }}</h2>
        <p>{{ libraryError }}</p>
        <AppButton variant="secondary" @click="libraryStore.loadLibraries">
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("library.scan") }}</span>
        </AppButton>
      </section>

      <section v-else-if="libraryStore.libraries.length === 0" class="library-state">
        <FolderOpen :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("library.emptyTitle") }}</h2>
        <p>{{ $t("library.emptyDescription") }}</p>
        <AppButton :disabled="libraryStore.isAdding" @click="libraryStore.addLibrary">
          <FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("library.emptyAction") }}</span>
        </AppButton>
      </section>

      <ul v-else class="library-grid" :aria-label="$t('library.title')">
        <li v-for="library in libraryStore.libraries" :key="library.id" class="library-card">
          <button class="library-card__open" type="button" @click="openLibrary(library)">
            <ThumbnailPreview
              class="library-card__preview"
              :record="coverAsset(library) ? thumbnailStore.recordsByImageId[coverAsset(library)!.id] : null"
              :label="coverAsset(library)?.filename ?? library.name"
              :generating-label="$t('home.thumbnailGenerating')"
              :unavailable-label="$t('home.thumbnailUnavailable')"
            />
            <span class="library-card__body">
              <span class="library-card__eyebrow">{{ $t("home.libraryLabel") }}</span>
              <span class="library-card__name" :title="library.name">{{ library.name }}</span>
              <span class="library-card__path" :title="library.path">{{ formatPathForDisplay(library.path) }}</span>
              <span class="library-card__meta">
                <span>{{ $t("library.added", { date: formatDate(library.createdAt) }) }}</span>
                <span>
                  <Image :size="14" :stroke-width="1.8" aria-hidden="true" />
                  {{ imageCountLabel(library) }}
                </span>
              </span>
            </span>
            <ChevronRight class="library-card__indicator" :size="18" :stroke-width="1.8" aria-hidden="true" />
          </button>

          <div class="library-card__actions">
            <AppButton
              variant="secondary"
              :disabled="libraryStore.scannerStates[library.id]?.status === 'scanning'"
              @click="libraryStore.scanLibrary(library.id)"
            >
              <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t("library.scan") }}</span>
            </AppButton>
            <AppDialog
              :open="removalCandidate?.id === library.id"
              :title="$t('library.removeTitle')"
              :description="$t('library.removeDescription', { name: library.name })"
              @update:open="updateRemovalDialog(library, $event)"
            >
              <template #trigger>
                <AppButton
                  variant="ghost"
                  :aria-label="$t('library.remove')"
                  :disabled="libraryStore.removingId === library.id"
                >
                  <Trash2 :size="18" :stroke-width="1.8" aria-hidden="true" />
                </AppButton>
              </template>
              <template #footer>
                <AppButton variant="secondary" @click="removalCandidate = null">
                  {{ $t("library.cancel") }}
                </AppButton>
                <AppButton
                  variant="danger"
                  :disabled="libraryStore.removingId === library.id"
                  @click="removeLibrary(library)"
                >
                  {{ libraryStore.removingId === library.id ? $t("library.removing") : $t("library.confirmRemove") }}
                </AppButton>
              </template>
            </AppDialog>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped lang="scss">
.library-page {
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.library-page__inner {
  display: grid;
  gap: var(--iv-space-6);
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.library-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-6);
}

.library-page__header h1,
.library-page__header p,
.library-state h2,
.library-state p {
  margin: 0;
}

.library-page__header h1 {
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.library-page__header p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.library-alert {
  margin: 0;
  padding: var(--iv-space-3) var(--iv-space-4);
  color: var(--iv-danger);
  background-color: var(--iv-danger-soft);
  border: 1px solid var(--iv-danger);
  border-radius: var(--iv-radius-sm);
}

.library-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--iv-space-4);
  margin: 0;
  padding: 0;
  list-style: none;
}

.library-card {
  position: relative;
  min-width: 0;
  overflow: hidden;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
  transition: background-color var(--iv-motion-fast) ease, border-color var(--iv-motion-fast) ease, box-shadow var(--iv-motion-fast) ease;
}

.library-card:hover,
.library-card:focus-within {
  background-color: var(--iv-surface-hover);
  border-color: var(--iv-accent);
  box-shadow: 0 0 0 1px var(--iv-accent-soft);
}

.library-card__open {
  position: relative;
  display: block;
  width: 100%;
  min-width: 0;
  padding: 0;
  color: inherit;
  font: inherit;
  text-align: left;
  background: transparent;
  border: 0;
  cursor: pointer;
  transition: background-color var(--iv-motion-fast) ease;
}

.library-card__open:hover {
  background-color: color-mix(in srgb, var(--iv-accent) 4%, transparent);
}

.library-card__open:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: calc(-1 * var(--iv-focus-ring-width));
}

.library-card__preview {
  width: 100%;
  aspect-ratio: 2.25;
}

.library-card__body {
  display: block;
  min-width: 0;
  padding: var(--iv-space-3);
  border-top: 1px solid var(--iv-border-subtle);
}

.library-card__eyebrow {
  display: block;
  margin-bottom: var(--iv-space-1);
  color: var(--iv-text-disabled);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.library-card__name,
.library-card__path {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.library-card__name {
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-22);
}

.library-card__path {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.library-card__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--iv-space-3);
  margin-top: var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.library-card__meta span {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-1);
}

.library-card__indicator {
  position: absolute;
  top: 50%;
  right: var(--iv-space-3);
  color: var(--iv-text-disabled);
  transform: translateY(-50%);
  transition: color var(--iv-motion-fast) ease, transform var(--iv-motion-fast) ease;
}

.library-card:hover .library-card__indicator,
.library-card:focus-within .library-card__indicator {
  color: var(--iv-accent);
  transform: translate(2px, -50%);
}

.library-card__actions {
  position: absolute;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--iv-space-2);
  padding: var(--iv-space-2) var(--iv-space-3) var(--iv-space-3);
  background-color: color-mix(in srgb, var(--iv-surface) 94%, transparent);
  border-top: 1px solid var(--iv-border-subtle);
  border-left: 1px solid var(--iv-border-subtle);
  border-top-left-radius: var(--iv-radius-sm);
  box-shadow: var(--iv-shadow-floating);
  transform: translateX(100%);
  transition: transform var(--iv-motion-normal) ease;
  z-index: 1;
}

.library-card:hover .library-card__actions,
.library-card:focus-within .library-card__actions {
  transform: translateX(0);
}

.library-card__actions :deep(.app-button) {
  white-space: nowrap;
}

@media (prefers-reduced-motion: reduce) {
  .library-card,
  .library-card__open,
  .library-card__indicator,
  .library-card__actions {
    transition: none;
  }

  .library-card__actions {
    transform: translateX(0);
  }
}

.library-state {
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

.library-state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  line-height: var(--iv-line-height-22);
}

.library-state p {
  max-width: 420px;
}

.library-state--error h2,
.library-state--error p {
  color: var(--iv-danger);
}

@media (max-width: 920px) {
  .library-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .library-page__header {
    flex-direction: column;
  }

  .library-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
