<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
  }>(),
  {
    variant: "primary",
    type: "button",
    disabled: false,
  },
);
</script>

<template>
  <button class="app-button" :class="`app-button--${variant}`" :type="type" :disabled="disabled">
    <slot />
  </button>
</template>

<style scoped lang="scss">
.app-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--iv-space-2);
  min-width: 0;
  height: var(--iv-control-height);
  padding: 0 var(--iv-space-4);
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-14);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-20);
  border: 1px solid transparent;
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    border-color var(--iv-motion-fast) ease, transform var(--iv-motion-fast) ease;
}

.app-button:active:not(:disabled) {
  transform: scale(var(--iv-press-scale));
}

.app-button:disabled {
  color: var(--iv-text-disabled);
  cursor: not-allowed;
}

.app-button--primary {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent);
}

.app-button--primary:hover:not(:disabled) {
  background-color: var(--iv-accent-hover, var(--iv-accent));
}

.app-button--primary:disabled {
  background-color: var(--iv-accent-soft);
}

.app-button--secondary {
  background-color: var(--iv-surface);
  border-color: var(--iv-border-subtle);
}

.app-button--secondary:hover:not(:disabled) {
  background-color: var(--iv-surface-hover);
  border-color: var(--iv-accent);
}

.app-button--secondary:disabled {
  background-color: var(--iv-bg-secondary);
  border-color: var(--iv-border-subtle);
}

.app-button--ghost {
  color: var(--iv-text-secondary);
}

.app-button--ghost:hover:not(:disabled) {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.app-button--danger {
  color: var(--iv-text-primary);
  background-color: var(--iv-danger);
}

.app-button--danger:hover:not(:disabled) {
  background-color: var(--iv-danger-soft);
}
</style>
