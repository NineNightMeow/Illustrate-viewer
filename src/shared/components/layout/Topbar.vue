<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ChevronLeft, ChevronRight, House, SlidersHorizontal } from "lucide-vue-next";
import AppInput from "../ui/AppInput.vue";
import AppTooltip from "../ui/AppTooltip.vue";
import IconButton from "../ui/IconButton.vue";
import { useSearchStore } from "../../../features/search/searchStore";
import { shortcutDispatcher } from "../../shortcuts/shortcutDispatcher";

const router = useRouter();
const searchStore = useSearchStore();
const searchInput = ref<InstanceType<typeof AppInput> | null>(null);
const searchQuery = computed({
  get: () => searchStore.query,
  set: (value: string) => { searchStore.query = value; },
});

async function openSearch() {
  await router.push({ name: "search" });
  if (searchStore.hasCriteria) await searchStore.search();
}

let unregisterShortcuts: (() => void) | null = null;

onMounted(() => {
  unregisterShortcuts = shortcutDispatcher.register("global", {
    "global.searchFocus": (event) => {
      event.preventDefault();
      searchInput.value?.focus();
    },
  });
});

onBeforeUnmount(() => {
  unregisterShortcuts?.();
});
</script>

<template>
  <header class="topbar iv-glass-surface" role="toolbar" :aria-label="$t('workspace.toolbar')">
    <div class="topbar__navigation">
      <IconButton :icon="ChevronLeft" :label="$t('topbar.back')" disabled />
      <IconButton :icon="ChevronRight" :label="$t('topbar.forward')" disabled />
      <AppTooltip :content="$t('topbar.goHome')">
        <RouterLink class="topbar__home-link" to="/" :aria-label="$t('topbar.goHome')">
          <House :size="20" :stroke-width="1.8" aria-hidden="true" />
        </RouterLink>
      </AppTooltip>
    </div>

    <div class="topbar__spacer" />

    <div class="topbar__search">
      <AppInput
        ref="searchInput"
        v-model="searchQuery"
        type="search"
        :placeholder="$t('topbar.search')"
        :aria-label="$t('topbar.search')"
        @keyup.enter="openSearch"
      />
    </div>
    <IconButton :icon="SlidersHorizontal" :label="$t('topbar.filters')" @click="openSearch" />
  </header>
</template>

<style scoped lang="scss">
.topbar {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-width: 0;
  min-height: var(--iv-topbar-height);
  padding: 0 var(--iv-space-4);
  background-color: var(--iv-chrome-surface);
  border-bottom: 1px solid var(--iv-border-subtle);
  z-index: var(--iv-layer-shell);
}

.topbar__navigation {
  display: flex;
  align-items: center;
  gap: var(--iv-space-1);
}

.topbar__spacer {
  flex: 1 1 auto;
  min-width: var(--iv-space-4);
}

.topbar__search {
  width: min(320px, 36vw);
}

.topbar__home-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-height);
  height: var(--iv-control-height);
  color: var(--iv-text-secondary);
  border-radius: var(--iv-radius-sm);
  transition: color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.topbar__home-link:hover {
  color: var(--iv-text-primary);
  background-color: var(--iv-surface-hover);
}
</style>
