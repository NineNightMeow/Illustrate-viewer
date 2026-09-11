<script setup lang="ts">
import { Check, Heart } from "lucide-vue-next";
import type { Component } from "vue";
import ThumbnailPreview from "../../library/components/ThumbnailPreview.vue";
import type { ImageAsset, ThumbnailRecord } from "../../library/types";

withDefaults(defineProps<{
  asset: ImageAsset;
  record: ThumbnailRecord | null;
  generatingLabel: string;
  unavailableLabel: string;
  selected?: boolean;
  active?: boolean;
  favorite?: boolean;
  favoritePending?: boolean;
  favoriteLabel: string;
  unfavoriteLabel: string;
  duplicateKind?: "exact" | "similar" | null;
  exactMatchLabel?: string;
  similarLabel?: string;
  secondaryActionLabel?: string;
  secondaryActionIcon?: Component | null;
  secondaryActionDanger?: boolean;
  tags?: { id: string; name: string }[];
}>(), {
  selected: false,
  active: false,
  favorite: false,
  favoritePending: false,
  duplicateKind: null,
  exactMatchLabel: "",
  similarLabel: "",
  secondaryActionLabel: "",
  secondaryActionIcon: null,
  secondaryActionDanger: false,
  tags: () => [],
});

const emit = defineEmits<{
  select: [asset: ImageAsset, event: MouseEvent];
  open: [asset: ImageAsset];
  "toggle-favorite": [asset: ImageAsset];
  "secondary-action": [asset: ImageAsset];
}>();
</script>

<template>
  <article
    class="gallery-item"
    :class="{
      'gallery-item--selected': selected,
      'gallery-item--active': active,
      'gallery-item--favorite': favorite,
    }"
    role="listitem"
  >
    <button
      class="gallery-item__button"
      type="button"
      :aria-label="asset.filename"
      :aria-pressed="selected"
      @click="emit('select', asset, $event)"
      @dblclick="emit('open', asset)"
    >
      <ThumbnailPreview
        class="gallery-item__preview"
        :record="record"
        :label="asset.filename"
        :generating-label="generatingLabel"
        :unavailable-label="unavailableLabel"
      />
      <span v-if="selected" class="gallery-item__check" aria-hidden="true">
        <Check :size="14" :stroke-width="2.2" />
      </span>
      <span v-if="duplicateKind" class="gallery-item__duplicate" :class="`gallery-item__duplicate--${duplicateKind}`">
        {{ duplicateKind === "exact" ? exactMatchLabel : similarLabel }}
      </span>
      <p class="gallery-item__filename" :title="asset.filename">{{ asset.filename }}</p>
      <div v-if="tags.length > 0" class="gallery-item__tags" :aria-label="$t('tags.imageTags')">
        <span
          v-for="tag in tags.slice(0, 3)"
          :key="tag.id"
          class="gallery-item__tag"
          :title="tag.name"
        >{{ tag.name }}</span>
        <span v-if="tags.length > 3" class="gallery-item__tag-more" :title="tags.slice(3).map((tag) => tag.name).join(', ')">+{{ tags.length - 3 }}</span>
      </div>
    </button>
    <button
      class="gallery-item__favorite"
      :class="{ 'gallery-item__favorite--with-secondary': secondaryActionIcon }"
      type="button"
      :aria-label="favorite ? unfavoriteLabel : favoriteLabel"
      :aria-pressed="favorite"
      :disabled="favoritePending"
      @click.stop="emit('toggle-favorite', asset)"
    >
      <Heart :size="16" :stroke-width="2" :fill="favorite ? 'currentColor' : 'none'" aria-hidden="true" />
    </button>
    <button
      v-if="secondaryActionIcon"
      class="gallery-item__secondary-action"
      :class="{ 'gallery-item__secondary-action--danger': secondaryActionDanger }"
      type="button"
      :aria-label="secondaryActionLabel"
      :title="secondaryActionLabel"
      @click.stop="emit('secondary-action', asset)"
    >
      <component :is="secondaryActionIcon" :size="16" :stroke-width="2" aria-hidden="true" />
    </button>
  </article>
</template>

<style scoped lang="scss">
.gallery-item {
  position: relative;
  min-width: 0;
}

.gallery-item__button {
  position: relative;
  display: grid;
  gap: var(--iv-space-2);
  width: 100%;
  padding: 0;
  color: inherit;
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: var(--iv-radius-md);
  cursor: pointer;
}

.gallery-item__preview {
  width: 100%;
  aspect-ratio: 1;
  overflow: hidden;
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.gallery-item__filename {
  margin: 0;
  overflow: hidden;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gallery-item__tags {
  display: flex;
  min-width: 0;
  gap: var(--iv-space-1);
  overflow: hidden;
}

.gallery-item__tag,
.gallery-item__tag-more {
  display: inline-block;
  max-width: 96px;
  padding: 1px var(--iv-space-2);
  overflow: hidden;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-11);
  line-height: var(--iv-line-height-16);
  text-overflow: ellipsis;
  white-space: nowrap;
  background-color: var(--iv-accent-soft);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-full);
}

.gallery-item__check {
  position: absolute;
  top: var(--iv-space-2);
  left: var(--iv-space-2);
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  color: var(--iv-text-primary);
  background-color: var(--iv-accent);
  border-radius: var(--iv-radius-round);
}

.gallery-item__duplicate {
  position: absolute;
  bottom: calc(var(--iv-line-height-18) + var(--iv-space-3));
  left: var(--iv-space-2);
  z-index: 1;
  max-width: calc(100% - var(--iv-space-4));
  padding: 2px var(--iv-space-2);
  overflow: hidden;
  color: var(--iv-overlay-text-primary);
  font-size: var(--iv-font-size-12);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-16);
  text-overflow: ellipsis;
  white-space: nowrap;
  background-color: var(--iv-overlay-surface-hud);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-round);
}

.gallery-item__duplicate--exact {
  border-color: var(--iv-accent);
}

.gallery-item__favorite {
  position: absolute;
  top: var(--iv-space-2);
  right: var(--iv-space-2);
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  padding: 0;
  color: var(--iv-text-secondary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-round);
  box-shadow: var(--iv-shadow-floating);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    border-color var(--iv-motion-fast) ease;
}

.gallery-item__favorite--with-secondary {
  right: calc(var(--iv-space-2) + var(--iv-control-small-height) + var(--iv-space-1));
}

.gallery-item__secondary-action {
  position: absolute;
  top: var(--iv-space-2);
  right: var(--iv-space-2);
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  padding: 0;
  color: var(--iv-text-secondary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-round);
  box-shadow: var(--iv-shadow-floating);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    border-color var(--iv-motion-fast) ease;
}

.gallery-item__secondary-action:hover {
  color: var(--iv-accent);
  background-color: var(--iv-accent-soft);
  border-color: var(--iv-accent);
}

.gallery-item__secondary-action--danger:hover {
  color: var(--iv-danger);
  background-color: var(--iv-danger-soft);
  border-color: var(--iv-danger);
}

.gallery-item__secondary-action:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.gallery-item__favorite:hover:not(:disabled) {
  color: var(--iv-accent);
  background-color: var(--iv-accent-soft);
  border-color: var(--iv-accent);
}

.gallery-item--favorite .gallery-item__favorite {
  color: var(--iv-accent);
  border-color: var(--iv-accent);
}

.gallery-item__favorite:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.gallery-item__favorite:disabled {
  cursor: wait;
  opacity: 0.6;
}

.gallery-item--selected .gallery-item__preview {
  border-color: var(--iv-accent);
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-accent-soft);
}

.gallery-item--active .gallery-item__filename {
  color: var(--iv-text-primary);
}

.gallery-item__button:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}
</style>
