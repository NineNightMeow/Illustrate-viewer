<script setup lang="ts">
import { ref } from "vue";

withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    disabled?: boolean;
    error?: string;
    type?: "text" | "search" | "email" | "password";
  }>(),
  {
    placeholder: "",
    disabled: false,
    error: "",
    type: "text",
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
  blur: [event: FocusEvent];
}>();

const inputElement = ref<HTMLInputElement | null>(null);

function updateValue(event: Event) {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

function focus() {
  inputElement.value?.focus();
}

defineExpose({ focus });
</script>

<template>
  <input
    ref="inputElement"
    class="app-input"
    :class="{ 'has-error': error }"
    :value="modelValue"
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    :aria-invalid="Boolean(error)"
    @input="updateValue"
    @blur="emit('blur', $event)"
  />
</template>

<style scoped lang="scss">
.app-input {
  width: 100%;
  height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
  outline: none;
  transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease,
    box-shadow var(--iv-motion-fast) ease;
}

.app-input::placeholder {
  color: var(--iv-text-secondary);
}

.app-input:hover:not(:disabled) {
  background-color: var(--iv-surface-hover);
}

.app-input:focus-visible {
  border-color: var(--iv-accent);
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-accent-soft);
}

.app-input.has-error {
  border-color: var(--iv-danger);
}

.app-input.has-error:focus-visible {
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-danger-soft);
}

.app-input:disabled {
  color: var(--iv-text-disabled);
  background-color: var(--iv-bg-secondary);
  cursor: not-allowed;
}
</style>
