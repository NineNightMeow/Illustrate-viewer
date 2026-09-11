<script setup lang="ts">
export type SegmentedOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

const props = defineProps<{
  modelValue: string;
  options: readonly SegmentedOption[];
  disabled?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function selectOption(option: SegmentedOption) {
  if (!props.disabled && !option.disabled) {
    emit("update:modelValue", option.value);
  }
}
</script>

<template>
  <div class="segmented-control" role="radiogroup">
    <button
      v-for="option in options"
      :key="option.value"
      class="segmented-control__option"
      :class="{ 'is-active': option.value === modelValue }"
      type="button"
      role="radio"
      :aria-checked="option.value === modelValue"
      :disabled="disabled || option.disabled"
      @click="selectOption(option)"
    >
      {{ option.label }}
    </button>
  </div>
</template>

<style scoped lang="scss">
.segmented-control {
  display: inline-flex;
  align-items: center;
  min-height: var(--iv-control-height);
  padding: var(--iv-control-inset);
  background-color: var(--iv-bg-secondary);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
}

.segmented-control__option {
      min-width: var(--iv-segmented-option-min-width);
      height: var(--iv-segmented-option-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-18);
  border-radius: var(--iv-radius-xs);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    transform var(--iv-motion-fast) ease;
}

.segmented-control__option:hover:not(:disabled) {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.segmented-control__option.is-active {
  color: var(--iv-text-primary);
  background-color: var(--iv-interactive-selected);
}

.segmented-control__option:active:not(:disabled) {
  transform: scale(var(--iv-press-scale));
}

.segmented-control__option:disabled {
  color: var(--iv-text-disabled);
  cursor: not-allowed;
}
</style>
