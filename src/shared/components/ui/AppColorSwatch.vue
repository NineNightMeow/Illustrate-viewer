<script setup lang="ts">
withDefaults(
  defineProps<{
    color: string;
    label: string;
    selected?: boolean;
    disabled?: boolean;
  }>(),
  {
    selected: false,
    disabled: false,
  },
);
</script>

<template>
  <button
    class="app-color-swatch"
    :class="{ 'is-selected': selected }"
    type="button"
    :style="{ backgroundColor: color }"
    :aria-label="label"
    :aria-pressed="selected"
    :disabled="disabled"
  />
</template>

<style scoped lang="scss">
.app-color-swatch {
  width: var(--iv-space-6);
  height: var(--iv-space-6);
  border: 2px solid transparent;
  border-radius: var(--iv-radius-round);
  cursor: pointer;
  transition: transform var(--iv-motion-fast) ease, border-color var(--iv-motion-fast) ease;
}

.app-color-swatch:hover:not(:disabled) {
  transform: scale(1.08);
}

.app-color-swatch.is-selected {
  border-color: var(--iv-text-primary);
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-accent-soft);
}

.app-color-swatch:disabled {
  cursor: not-allowed;
}
</style>
