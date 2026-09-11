<script setup lang="ts">
import {
  TooltipContent,
  TooltipPortal,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
} from "reka-ui";

withDefaults(
  defineProps<{
    content: string;
    side?: "top" | "right" | "bottom" | "left";
    disabled?: boolean;
  }>(),
  {
    side: "top",
    disabled: false,
  },
);
</script>

<template>
  <TooltipProvider :delay-duration="500">
    <TooltipRoot :disabled="disabled">
      <TooltipTrigger as-child>
        <slot name="trigger">
          <slot />
        </slot>
      </TooltipTrigger>
      <TooltipPortal>
        <TooltipContent class="app-tooltip iv-glass-surface" :side="side" :side-offset="8">
          {{ content }}
        </TooltipContent>
      </TooltipPortal>
    </TooltipRoot>
  </TooltipProvider>
</template>

<style scoped lang="scss">
.app-tooltip {
  z-index: var(--iv-layer-tooltip);
  max-width: var(--iv-tooltip-max-width);
  padding: var(--iv-space-2) var(--iv-space-3);
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  background-color: var(--iv-floating-surface);
  border: 1px solid var(--iv-floating-border);
  border-radius: var(--iv-radius-sm);
  box-shadow: var(--iv-shadow-floating);
  animation: app-tooltip-in var(--iv-motion-fast) ease-out;
}

@keyframes app-tooltip-in {
  from {
    opacity: 0;
    transform: translateY(var(--iv-space-1));
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
