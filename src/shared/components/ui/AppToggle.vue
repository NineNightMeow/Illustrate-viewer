<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    disabled?: boolean;
    label: string;
  }>(),
  {
    disabled: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();

function toggle() {
  if (!props.disabled) {
    emit("update:modelValue", !props.modelValue);
  }
}
</script>

<template>
  <button
    class="app-toggle"
    :class="{ 'is-on': modelValue }"
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :aria-label="label"
    :disabled="disabled"
    @click="toggle"
  >
    <span class="app-toggle__thumb" aria-hidden="true" />
  </button>
</template>

<style scoped lang="scss">
.app-toggle {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex: 0 0 var(--iv-toggle-width);
  width: var(--iv-toggle-width);
  height: var(--iv-toggle-height);
  padding: var(--iv-control-inset);
  background-color: var(--iv-border-subtle);
  border: 1px solid transparent;
  border-radius: var(--iv-radius-round);
  cursor: pointer;
  transition: background-color var(--iv-motion-normal) ease, border-color var(--iv-motion-normal) ease;
}

.app-toggle__thumb {
  display: block;
  width: var(--iv-toggle-thumb-size);
  height: var(--iv-toggle-thumb-size);
  background-color: var(--iv-text-primary);
  border-radius: var(--iv-radius-round);
  transform: translateX(0);
  transition: background-color var(--iv-motion-normal) ease, transform var(--iv-motion-normal) ease;
}

.app-toggle.is-on {
  background-color: var(--iv-accent);
}

.app-toggle:hover:not(:disabled) {
  border-color: var(--iv-accent);
}

.app-toggle.is-on:hover:not(:disabled) {
  background-color: var(--iv-accent-hover);
}

.app-toggle.is-on .app-toggle__thumb {
  transform: translateX(var(--iv-toggle-thumb-travel));
}

.app-toggle:active:not(:disabled) {
  transform: scale(var(--iv-press-scale));
}

.app-toggle:disabled {
  background-color: var(--iv-bg-secondary);
  border-color: var(--iv-border-subtle);
  cursor: not-allowed;
}

.app-toggle:disabled .app-toggle__thumb {
  background-color: var(--iv-text-disabled);
}
</style>
