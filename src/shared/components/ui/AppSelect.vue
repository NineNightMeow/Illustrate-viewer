<script setup lang="ts">
import { ChevronDown } from "lucide-vue-next";

export type AppSelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

withDefaults(
  defineProps<{
    modelValue: string;
    options: readonly AppSelectOption[];
    disabled?: boolean;
    ariaLabel?: string;
  }>(),
  {
    disabled: false,
    ariaLabel: undefined,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function updateValue(event: Event) {
  emit("update:modelValue", (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <label class="app-select">
    <select
      :value="modelValue"
      :disabled="disabled"
      :aria-label="ariaLabel"
      @change="updateValue"
    >
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
        :disabled="option.disabled"
      >
        {{ option.label }}
      </option>
    </select>
    <ChevronDown class="app-select__icon" :size="16" :stroke-width="1.8" aria-hidden="true" />
  </label>
</template>

<style scoped lang="scss">
.app-select {
  position: relative;
  display: inline-flex;
  align-items: center;
  width: var(--iv-settings-control-width);
  height: var(--iv-control-height);
}

.app-select select {
  width: 100%;
  height: 100%;
  padding: 0 var(--iv-space-8) 0 var(--iv-space-3);
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-14);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
  outline: none;
  appearance: none;
  transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    box-shadow var(--iv-motion-fast) ease;
}

.app-select select:hover:not(:disabled) {
  background-color: var(--iv-surface-hover);
}

.app-select select:focus-visible {
  border-color: var(--iv-accent);
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-accent-soft);
}

.app-select select:disabled {
  color: var(--iv-text-disabled);
  background-color: var(--iv-bg-secondary);
  cursor: not-allowed;
}

.app-select__icon {
  position: absolute;
  right: var(--iv-space-3);
  color: var(--iv-text-secondary);
  pointer-events: none;
}
</style>
