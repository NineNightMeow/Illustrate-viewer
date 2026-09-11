<script setup lang="ts">
import type { Component } from "vue";
import { ChevronRight } from "lucide-vue-next";

withDefaults(
  defineProps<{
    title: string;
    icon: Component;
    showViewAll?: boolean;
  }>(),
  {
    showViewAll: true,
  },
);
</script>

<template>
  <section class="home-section">
    <header class="home-section__header">
      <div class="home-section__title">
        <component :is="icon" :size="20" :stroke-width="1.8" aria-hidden="true" />
        <h2>{{ title }}</h2>
      </div>
      <button v-if="showViewAll" class="home-section__view-all" type="button">
        <span>{{ $t("home.viewAll") }}</span>
        <ChevronRight :size="16" :stroke-width="1.8" aria-hidden="true" />
      </button>
    </header>
    <div class="home-section__content">
      <slot />
    </div>
  </section>
</template>

<style scoped lang="scss">
.home-section {
  min-width: 0;
}

.home-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-4);
  margin-bottom: var(--iv-space-3);
}

.home-section__title,
.home-section__view-all {
  display: inline-flex;
  align-items: center;
}

.home-section__title {
  gap: var(--iv-space-2);
  min-width: 0;
  color: var(--iv-text-primary);
}

.home-section__title h2 {
  min-width: 0;
  overflow: hidden;
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-22);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.home-section__view-all {
  gap: var(--iv-space-1);
  flex: 0 0 auto;
  min-height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.home-section__view-all:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.home-section__content {
  min-width: 0;
}
</style>
