<script setup lang="ts">
import type { Component } from "vue";
import { useRoute } from "vue-router";
import {
  Folder,
  FolderOpen,
  Heart,
  House,
  Library,
  PanelLeftClose,
  PanelLeftOpen,
  Settings,
  Sparkles,
  Tags,
} from "lucide-vue-next";
import { useAppStore } from "../../../app/appStore";
import { useLibraryStore } from "../../../features/library/libraryStore";
import AppTooltip from "../ui/AppTooltip.vue";

type NavigationItem = {
  labelKey: string;
  path?: string;
  icon: Component;
};

const appStore = useAppStore();
const libraryStore = useLibraryStore();
const route = useRoute();

const navigationItems: NavigationItem[] = [
  { labelKey: "navigation.home", path: "/", icon: House },
  { labelKey: "navigation.library", path: "/library", icon: Library },
  { labelKey: "navigation.favorites", path: "/favorites", icon: Heart },
  { labelKey: "navigation.collections", path: "/collections", icon: Folder },
  { labelKey: "navigation.smartCollections", path: "/smart-collections", icon: Sparkles },
  { labelKey: "navigation.tags", path: "/tags", icon: Tags },
  { labelKey: "navigation.openFolder", icon: FolderOpen },
];

function isActive(path: string) {
  if (path === "/library" || path === "/smart-collections") {
    return route.path === path || route.path.startsWith(`${path}/`);
  }
  return route.path === path;
}

function toggleSidebar() {
  appStore.sidebarCollapsed = !appStore.sidebarCollapsed;
}

function openFolder() {
  void libraryStore.addLibrary();
}
</script>

<template>
  <aside class="sidebar iv-glass-surface" :class="{ 'is-collapsed': appStore.sidebarCollapsed }">
    <nav class="sidebar-nav" :aria-label="$t('navigation.label')">
      <AppTooltip
        v-for="item in navigationItems"
        :key="item.labelKey"
        :content="$t(item.labelKey)"
        :disabled="!appStore.sidebarCollapsed"
      >
        <RouterLink
          v-if="item.path"
          class="sidebar-link"
          :class="{ 'is-active': isActive(item.path) }"
          :to="item.path"
          :aria-label="$t(item.labelKey)"
          :aria-current="isActive(item.path) ? 'page' : undefined"
        >
          <component :is="item.icon" :size="20" :stroke-width="1.8" aria-hidden="true" />
          <span v-if="!appStore.sidebarCollapsed" class="sidebar-label">
            {{ $t(item.labelKey) }}
          </span>
        </RouterLink>
        <button
          v-else
          class="sidebar-link sidebar-link--open-folder"
          type="button"
          :aria-label="$t(item.labelKey)"
          :disabled="libraryStore.isAdding"
          @click="openFolder"
        >
          <component :is="item.icon" :size="20" :stroke-width="1.8" aria-hidden="true" />
          <span v-if="!appStore.sidebarCollapsed" class="sidebar-label">
            {{ $t(item.labelKey) }}
          </span>
        </button>
      </AppTooltip>
    </nav>

    <div class="sidebar-footer">
      <AppTooltip :content="$t('navigation.settings')" :disabled="!appStore.sidebarCollapsed">
        <RouterLink
          class="sidebar-link"
          :class="{ 'is-active': isActive('/settings') }"
          to="/settings"
          :aria-label="$t('navigation.settings')"
          :aria-current="isActive('/settings') ? 'page' : undefined"
        >
          <Settings :size="20" :stroke-width="1.8" aria-hidden="true" />
          <span v-if="!appStore.sidebarCollapsed" class="sidebar-label">
            {{ $t("navigation.settings") }}
          </span>
        </RouterLink>
      </AppTooltip>

      <AppTooltip
        :content="appStore.sidebarCollapsed ? $t('sidebar.expand') : $t('sidebar.collapse')"
      >
        <button
          class="sidebar-toggle"
          type="button"
          :aria-label="appStore.sidebarCollapsed ? $t('sidebar.expand') : $t('sidebar.collapse')"
          @click="toggleSidebar"
        >
          <PanelLeftOpen
            v-if="appStore.sidebarCollapsed"
            :size="18"
            :stroke-width="1.8"
            aria-hidden="true"
          />
          <PanelLeftClose v-else :size="18" :stroke-width="1.8" aria-hidden="true" />
        </button>
      </AppTooltip>
    </div>
  </aside>
</template>

<style scoped lang="scss">
.sidebar {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  padding: var(--iv-space-6) var(--iv-space-2) var(--iv-space-4);
  color: var(--iv-text-secondary);
  background-color: var(--iv-chrome-sidebar-surface);
  border-right: 1px solid var(--iv-border-subtle);
  transition: padding var(--iv-motion-normal) ease;
}

.sidebar-nav,
.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: var(--iv-space-2);
  min-width: 0;
}

.sidebar-footer {
  margin-top: auto;
  padding-top: var(--iv-space-4);
  border-top: 1px solid var(--iv-border-subtle);
}

.sidebar-link,
.sidebar-toggle {
  display: flex;
  align-items: center;
  min-width: 0;
  width: 100%;
  height: var(--iv-control-height);
  color: inherit;
  border-radius: var(--iv-radius-sm);
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.sidebar-link {
  gap: var(--iv-space-3);
  padding: 0 var(--iv-space-3);
}

.sidebar-link::before {
  flex: 0 0 2px;
  width: 2px;
  height: 20px;
  content: "";
  border-radius: var(--iv-radius-round);
  background-color: transparent;
}

.sidebar-link:hover,
.sidebar-toggle:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}

.sidebar-link.is-active {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent-soft);
}

.sidebar-link.is-active::before {
  background-color: var(--iv-accent);
}

.sidebar-link--open-folder {
  margin-top: var(--iv-space-2);
  color: var(--iv-text-primary);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
}

.sidebar-link--open-folder::before {
  display: none;
}

.sidebar-link--open-folder:hover:not(:disabled) {
  border-color: var(--iv-accent);
}

.sidebar-link:disabled {
  color: var(--iv-text-disabled);
  cursor: not-allowed;
}

.sidebar-label {
  min-width: 0;
  overflow: hidden;
  font-size: var(--iv-font-size-15);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-toggle {
  justify-content: center;
  cursor: pointer;
}

.sidebar-link:active,
.sidebar-toggle:active {
  transform: scale(var(--iv-press-scale));
}

.sidebar.is-collapsed {
  padding-right: var(--iv-space-2);
  padding-left: var(--iv-space-2);
}

.sidebar.is-collapsed .sidebar-link {
  justify-content: center;
  gap: 0;
  padding-right: 0;
  padding-left: 0;
}

.sidebar.is-collapsed .sidebar-link::before {
  display: none;
}
</style>
