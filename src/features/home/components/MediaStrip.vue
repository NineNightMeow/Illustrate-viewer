<script setup lang="ts">
withDefaults(
  defineProps<{
    count?: number;
    label: string;
    compact?: boolean;
  }>(),
  {
    count: 4,
    compact: false,
  },
);
</script>

<template>
  <div class="media-strip" :class="{ 'is-compact': compact }" role="img" :aria-label="label">
    <span v-for="index in count" :key="index" class="media-strip__tile" :class="`media-strip__tile--${(index - 1) % 3}`" />
  </div>
</template>

<style scoped lang="scss">
.media-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--iv-space-1);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background-color: var(--iv-bg-primary);
}

.media-strip.is-compact {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.media-strip__tile {
  position: relative;
  min-width: 0;
  min-height: 0;
  background-color: var(--iv-surface-hover);
}

.media-strip__tile::before,
.media-strip__tile::after {
  position: absolute;
  content: "";
  border: 1px solid var(--iv-placeholder-line);
}

.media-strip__tile::before {
  inset: 18% 16%;
  border-radius: var(--iv-radius-xs);
  transform: rotate(-5deg);
}

.media-strip__tile::after {
  right: 18%;
  bottom: 22%;
  left: 22%;
  height: 1px;
  border: 0;
  background-color: var(--iv-placeholder-line-strong);
}

.media-strip__tile--1 {
  background-color: var(--iv-surface);
}

.media-strip__tile--2 {
  background-color: var(--iv-bg-secondary);
}
</style>
