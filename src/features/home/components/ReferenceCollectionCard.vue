<script setup lang="ts">
import { MoreHorizontal } from "lucide-vue-next";
import IconButton from "../../../shared/components/ui/IconButton.vue";
import MediaStrip from "./MediaStrip.vue";

defineProps<{
  title: string;
  count: string;
  imageLabel: string;
}>();
</script>

<template>
  <article class="reference-card">
    <MediaStrip compact :label="imageLabel" />
    <div class="reference-card__copy">
      <h3>{{ title }}</h3>
      <p>{{ count }}</p>
    </div>
    <IconButton
      class="reference-card__menu"
      :icon="MoreHorizontal"
      :label="$t('home.moreActions')"
    />
  </article>
</template>

<style scoped lang="scss">
.reference-card {
  position: relative;
  display: grid;
  grid-template-columns: var(--iv-home-reference-preview-width) minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--iv-space-3);
  min-width: 0;
  min-height: 88px;
  padding: var(--iv-space-2);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.reference-card :deep(.media-strip) {
  width: var(--iv-home-reference-preview-width);
  height: var(--iv-home-reference-preview-height);
  border-radius: var(--iv-radius-sm);
}

.reference-card__copy {
  min-width: 0;
}

.reference-card__copy h3 {
  overflow: hidden;
  margin: 0;
  font-size: var(--iv-font-size-14);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reference-card__copy p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.reference-card__menu {
  color: var(--iv-text-secondary);
}

@media (max-width: 1180px) {
  .reference-card {
    grid-template-columns: var(--iv-home-reference-preview-narrow-width) minmax(0, 1fr) auto;
  }

  .reference-card :deep(.media-strip) {
    width: var(--iv-home-reference-preview-narrow-width);
  }
}
</style>
