<script setup lang="ts">
import { computed } from "vue";
import { CalendarDays, Image } from "lucide-vue-next";
import { formatPathForDisplay } from "../../../shared/utils/formatPathForDisplay";
import ThumbnailPreview from "../../library/components/ThumbnailPreview.vue";
import type { Library, ThumbnailRecord } from "../../library/types";

const props = defineProps<{
  library: Library;
  createdLabel: string;
  imageCountLabel: string;
  thumbnailRecord: ThumbnailRecord | null;
  thumbnailLabel: string;
  generatingLabel: string;
  unavailableLabel: string;
}>();
const emit = defineEmits<{
  open: [library: Library];
}>();

const displayPath = computed(() => formatPathForDisplay(props.library.path));
</script>

<template>
  <button class="library-card" type="button" @click="emit('open', library)">
    <div class="library-card__preview">
      <ThumbnailPreview
        :record="thumbnailRecord"
        :label="thumbnailLabel"
        :generating-label="generatingLabel"
        :unavailable-label="unavailableLabel"
      />
    </div>
    <div class="library-card__body">
      <span class="library-card__eyebrow">{{ $t("home.libraryLabel") }}</span>
      <h3 :title="library.name">{{ library.name }}</h3>
      <p class="library-card__path" :title="library.path">{{ displayPath }}</p>
      <div class="library-card__meta">
        <p>
          <CalendarDays :size="14" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ createdLabel }}</span>
        </p>
        <p>
          <Image :size="14" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ imageCountLabel }}</span>
        </p>
      </div>
    </div>
  </button>
</template>

<style scoped lang="scss">
.library-card {
  min-width: 0;
  width: 100%;
  padding: 0;
  overflow: hidden;
  color: inherit;
  font: inherit;
  text-align: left;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
  cursor: pointer;
  transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.library-card:hover {
  background-color: var(--iv-surface-hover);
  border-color: var(--iv-accent);
}

.library-card:active {
  transform: scale(var(--iv-press-scale));
}

.library-card:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.library-card__preview {
  position: relative;
  aspect-ratio: 2.25;
  overflow: hidden;
}

.library-card__preview :deep(.thumbnail-preview) {
  width: 100%;
  height: 100%;
}

.library-card__body {
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

.library-card__body h3 {
  overflow: hidden;
  margin: 0;
  font-size: var(--iv-font-size-14);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.library-card__path {
  margin-top: var(--iv-space-1);
  overflow: hidden;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.library-card__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--iv-space-1);
  margin-top: var(--iv-space-2);
}

.library-card__meta p {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-1);
  margin: 0;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}
</style>
