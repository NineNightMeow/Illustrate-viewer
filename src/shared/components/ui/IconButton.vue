<script setup lang="ts">
import type { Component } from "vue";
import AppTooltip from "./AppTooltip.vue";

withDefaults(
  defineProps<{
    icon: Component;
    label: string;
    disabled?: boolean;
    size?: number;
  }>(),
  {
    disabled: false,
    size: 18,
  },
);

const emit = defineEmits<{
  click: [];
}>();
</script>

<template>
  <AppTooltip :content="label" :disabled="disabled">
    <button
      class="icon-button"
      type="button"
      :aria-label="label"
      :disabled="disabled"
      @click="emit('click')"
    >
      <component :is="icon" :size="size" :stroke-width="1.8" aria-hidden="true" />
    </button>
  </AppTooltip>
</template>

<style scoped lang="scss">
.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 var(--iv-control-height);
  width: var(--iv-control-height);
  height: var(--iv-control-height);
  color: var(--iv-text-secondary);
  background-color: transparent;
  border: 1px solid transparent;
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    border-color var(--iv-motion-fast) ease, transform var(--iv-motion-fast) ease;
}

.icon-button:hover:not(:disabled) {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
  border-color: var(--iv-border-subtle);
}

.icon-button:active:not(:disabled) {
  transform: scale(var(--iv-press-scale));
}

.icon-button:disabled {
  color: var(--iv-text-disabled);
  cursor: not-allowed;
}
</style>
