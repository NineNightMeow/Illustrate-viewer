<script setup lang="ts">
import {
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
} from "reka-ui";

withDefaults(
  defineProps<{
    open?: boolean;
    side?: "top" | "right" | "bottom" | "left";
    align?: "start" | "center" | "end";
  }>(),
  {
    open: undefined,
    side: "bottom",
    align: "start",
  },
);

const emit = defineEmits<{
  "update:open": [value: boolean];
}>();
</script>

<template>
  <PopoverRoot :open="open" @update:open="emit('update:open', $event)">
    <PopoverTrigger as-child>
      <slot name="trigger" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="app-popover iv-glass-surface"
        :side="side"
        :align="align"
        :side-offset="8"
      >
        <slot />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<style scoped lang="scss">
.app-popover {
  z-index: var(--iv-layer-popover);
  width: max-content;
  max-width: var(--iv-popover-max-width);
  padding: var(--iv-floating-padding);
  color: var(--iv-text-primary);
  background-color: var(--iv-floating-surface);
  border: 1px solid var(--iv-floating-border);
  border-radius: var(--iv-radius-md);
  box-shadow: var(--iv-shadow-floating);
  animation: app-popover-in var(--iv-motion-fast) ease-out;
}

@keyframes app-popover-in {
  from {
    opacity: 0;
    transform: translateY(calc(-1 * var(--iv-space-1)));
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
