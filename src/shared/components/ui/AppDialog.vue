<script setup lang="ts">
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
  DialogTrigger,
} from "reka-ui";

withDefaults(
  defineProps<{
    open?: boolean;
    title: string;
    description?: string;
  }>(),
  {
    open: undefined,
    description: undefined,
  },
);

const emit = defineEmits<{
  "update:open": [value: boolean];
}>();
</script>

<template>
  <DialogRoot :open="open" @update:open="emit('update:open', $event)">
    <DialogTrigger as-child>
      <slot name="trigger" />
    </DialogTrigger>
    <DialogPortal>
      <DialogOverlay class="app-dialog__overlay" />
      <DialogContent class="app-dialog iv-glass-surface">
        <DialogTitle class="app-dialog__title">{{ title }}</DialogTitle>
        <DialogDescription v-if="description" class="app-dialog__description">
          {{ description }}
        </DialogDescription>
        <div class="app-dialog__body">
          <slot />
        </div>
        <div v-if="$slots.footer" class="app-dialog__footer">
          <slot name="footer" />
        </div>
        <DialogClose v-if="$slots.close" as-child>
          <slot name="close" />
        </DialogClose>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<style scoped lang="scss">
.app-dialog__overlay {
  position: fixed;
  inset: 0;
  z-index: var(--iv-layer-dialog);
  background-color: var(--iv-app-background-overlay);
  animation: app-dialog-fade var(--iv-motion-fast) ease-out;
}

.app-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: calc(var(--iv-layer-dialog) + 1);
  width: min(var(--iv-dialog-width), calc(100vw - var(--iv-space-8)));
  padding: var(--iv-space-6);
  color: var(--iv-text-primary);
  background-color: var(--iv-floating-surface);
  border: 1px solid var(--iv-floating-border);
  border-radius: var(--iv-radius-lg);
  box-shadow: var(--iv-shadow-floating);
  transform: translate(-50%, -50%);
  animation: app-dialog-in var(--iv-motion-normal) ease-out;
}

.app-dialog__title {
  margin: 0;
  font-size: var(--iv-font-size-20);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-28);
}

.app-dialog__description {
  margin: var(--iv-space-2) 0 0;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.app-dialog__body {
  margin-top: var(--iv-space-4);
}

.app-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--iv-space-2);
  margin-top: var(--iv-space-6);
}

@keyframes app-dialog-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes app-dialog-in {
  from { opacity: 0; transform: translate(-50%, calc(-50% + var(--iv-space-2))); }
  to { opacity: 1; transform: translate(-50%, -50%); }
}
</style>
