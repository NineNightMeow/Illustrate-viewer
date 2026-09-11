<script setup lang="ts">
import type { Component } from "vue";
import {
  Info,
  Keyboard,
  Palette,
} from "lucide-vue-next";

export type SettingsCategory = {
  key: string;
  labelKey: string;
  icon: Component;
};

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const categories: readonly SettingsCategory[] = [
  { key: "appearance", labelKey: "settings.appearance", icon: Palette },
  { key: "shortcuts", labelKey: "settings.shortcuts", icon: Keyboard },
  { key: "about", labelKey: "settings.about", icon: Info },
];

function selectCategory(category: SettingsCategory) {
  emit("update:modelValue", category.key);
}
</script>

<template>
  <nav class="settings-sidebar" :aria-label="$t('settings.title')">
    <button
      v-for="category in categories"
      :key="category.key"
      class="settings-sidebar__item"
      :class="{ 'is-active': props.modelValue === category.key }"
      type="button"
      :aria-current="props.modelValue === category.key ? 'page' : undefined"
      @click="selectCategory(category)"
    >
      <component :is="category.icon" :size="20" :stroke-width="1.8" aria-hidden="true" />
      <span>{{ $t(category.labelKey) }}</span>
    </button>
  </nav>
</template>

<style scoped lang="scss">
.settings-sidebar {
  display: grid;
  align-content: start;
  gap: var(--iv-space-2);
  width: var(--iv-settings-nav-width);
  min-width: 0;
}

.settings-sidebar__item {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  width: 100%;
  min-height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-15);
  line-height: var(--iv-line-height-20);
  text-align: left;
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.settings-sidebar__item:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.settings-sidebar__item.is-active {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent-soft);
}

.settings-sidebar__item span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
