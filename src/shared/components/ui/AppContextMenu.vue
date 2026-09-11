<script setup lang="ts">
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuRoot,
  ContextMenuTrigger,
} from "reka-ui";

export type ContextMenuOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

const props = withDefaults(
  defineProps<{
    items?: readonly ContextMenuOption[];
  }>(),
  {
    items: () => [],
  },
);

const emit = defineEmits<{
  select: [value: string];
}>();
</script>

<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child>
      <slot name="trigger" />
    </ContextMenuTrigger>
    <ContextMenuPortal>
      <ContextMenuContent class="app-context-menu iv-glass-surface">
        <slot>
          <ContextMenuItem v-for="item in props.items" :key="item.value" class="app-context-menu__item"
            :disabled="item.disabled" @select="emit('select', item.value)">
            {{ item.label }}
          </ContextMenuItem>
        </slot>
      </ContextMenuContent>
    </ContextMenuPortal>
  </ContextMenuRoot>
</template>

<style scoped lang="scss">
.app-context-menu {
  z-index: var(--iv-layer-popover);
  min-width: var(--iv-context-menu-width);
  padding: var(--iv-floating-padding);
  color: var(--iv-text-primary);
  background-color: var(--iv-floating-surface);
  border: 1px solid var(--iv-floating-border);
  border-radius: var(--iv-radius-md);
  box-shadow: var(--iv-shadow-floating);
  animation: app-context-menu-in var(--iv-motion-fast) ease-out;
}

.app-context-menu__item {
  display: flex;
  align-items: center;
  min-height: var(--iv-menu-item-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  border-radius: var(--iv-radius-xs);
  outline: none;
  cursor: pointer;
}

.app-context-menu__item[data-highlighted] {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.app-context-menu__item[data-disabled] {
  color: var(--iv-text-disabled);
  cursor: not-allowed;
}

@keyframes app-context-menu-in {
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
