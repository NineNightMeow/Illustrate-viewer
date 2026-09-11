<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { ImageOff, LoaderCircle } from "lucide-vue-next";
import { computed, ref, watch } from "vue";
import ThumbnailPlaceholder from "../../../shared/components/ui/ThumbnailPlaceholder.vue";
import type { ThumbnailRecord } from "../types";

const props = withDefaults(
  defineProps<{
    record?: ThumbnailRecord | null;
    label: string;
    generatingLabel: string;
    unavailableLabel: string;
    compact?: boolean;
  }>(),
  {
    record: null,
    compact: false,
  },
);

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const imageFailed = ref(false);
const isGenerating = computed(
  () => props.record?.status === "pending" || props.record?.status === "generating",
);
const isUnavailable = computed(() => props.record?.status === "failed" || imageFailed.value);
const source = computed(() =>
  isTauriEnvironment && props.record?.status === "completed" && props.record.thumbnailPath && !imageFailed.value
    ? convertFileSrc(props.record.thumbnailPath)
    : null,
);

watch(
  () => props.record?.thumbnailPath,
  () => {
    imageFailed.value = false;
  },
);
</script>

<template>
  <div class="thumbnail-preview" :class="{ 'thumbnail-preview--compact': compact }">
    <img
      v-if="source"
      :src="source"
      :alt="label"
      loading="lazy"
      decoding="async"
      @error="imageFailed = true"
    />
    <ThumbnailPlaceholder v-else :label="label" />

    <span v-if="isGenerating" class="thumbnail-preview__status" role="status">
      <LoaderCircle :size="16" :stroke-width="1.8" aria-hidden="true" />
      <span>{{ generatingLabel }}</span>
    </span>
    <span v-else-if="isUnavailable" class="thumbnail-preview__status thumbnail-preview__status--failed" role="status">
      <ImageOff :size="16" :stroke-width="1.8" aria-hidden="true" />
      <span>{{ unavailableLabel }}</span>
    </span>
  </div>
</template>

<style scoped lang="scss">
.thumbnail-preview {
  position: relative;
  display: block;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background-color: var(--iv-thumbnail-matte-surface);
}

.thumbnail-preview > img,
.thumbnail-preview > :deep(.thumbnail-placeholder) {
  display: block;
  width: 100%;
  height: 100%;
}

.thumbnail-preview > img {
  object-fit: contain;
}

.thumbnail-preview__status {
  position: absolute;
  right: var(--iv-space-2);
  bottom: var(--iv-space-2);
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-1);
  max-width: calc(100% - var(--iv-space-4));
  min-height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  overflow: hidden;
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  text-overflow: ellipsis;
  white-space: nowrap;
  background-color: color-mix(in srgb, var(--iv-bg-primary) 82%, transparent);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-round);
}

.thumbnail-preview__status > svg {
  flex: 0 0 auto;
  animation: thumbnail-preview-spin 900ms linear infinite;
}

.thumbnail-preview__status--failed {
  color: var(--iv-danger);
}

.thumbnail-preview__status--failed > svg {
  animation: none;
}

.thumbnail-preview--compact .thumbnail-preview__status {
  right: var(--iv-space-1);
  bottom: var(--iv-space-1);
  min-height: 0;
  padding: var(--iv-space-1);
  border-radius: var(--iv-radius-round);
}

.thumbnail-preview--compact .thumbnail-preview__status span {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}

@keyframes thumbnail-preview-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .thumbnail-preview__status > svg {
    animation: none;
  }
}
</style>
