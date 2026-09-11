<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    disabled?: boolean;
  }>(),
  {
    description: undefined,
    disabled: false,
  },
);
</script>

<template>
  <div class="settings-row" :class="{ 'is-disabled': disabled, 'has-description': description }">
    <div class="settings-row__copy">
      <h2>{{ title }}</h2>
      <p v-if="description">{{ description }}</p>
    </div>
    <div class="settings-row__control">
      <slot />
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-6);
  min-height: var(--iv-settings-row-height);
  padding: var(--iv-space-1) var(--iv-space-4);
  border-bottom: 1px solid var(--iv-border-subtle);
}

.settings-row.has-description {
  min-height: var(--iv-settings-description-row-height);
}

.settings-row:last-child {
  border-bottom: 0;
}

.settings-row__copy {
  min-width: 0;
}

.settings-row__copy h2 {
  margin: 0;
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-regular);
  line-height: var(--iv-line-height-22);
}

.settings-row__copy p {
  max-width: 640px;
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.settings-row__control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex: 0 0 auto;
}

.settings-row.is-disabled {
  opacity: 0.42;
}
</style>
