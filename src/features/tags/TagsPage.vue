<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Tag, Tags, Pencil, Plus, Trash2 } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import AppInput from "../../shared/components/ui/AppInput.vue";
import { useTagStore } from "./tagStore";
import type { TagErrorCode } from "./tagStore";

const tagStore = useTagStore();
const createDialogOpen = ref(false);
const renameCandidate = ref<{ id: string; name: string } | null>(null);
const deleteCandidate = ref<{ id: string; name: string } | null>(null);
const createName = ref("");
const renameName = ref("");
const createError = ref<TagErrorCode | null>(null);
const renameError = ref<TagErrorCode | null>(null);

onMounted(() => {
  void tagStore.load();
});

function updateCreateDialog(open: boolean) {
  createDialogOpen.value = open;
  if (open) {
    createName.value = "";
    createError.value = null;
  }
}

function updateRenameDialog(tag: { id: string; name: string }, open: boolean) {
  renameCandidate.value = open ? tag : null;
  if (open) {
    renameName.value = tag.name;
    renameError.value = null;
  }
}

async function createTag() {
  const tag = await tagStore.create(createName.value);
  if (tag) {
    createDialogOpen.value = false;
    return;
  }
  createError.value = tagStore.error;
}

async function renameTag() {
  if (!renameCandidate.value) return;
  const tag = await tagStore.rename(renameCandidate.value.id, renameName.value);
  if (tag) {
    renameCandidate.value = null;
    return;
  }
  renameError.value = tagStore.error;
}

async function deleteTag() {
  if (!deleteCandidate.value) return;
  if (await tagStore.remove(deleteCandidate.value.id)) deleteCandidate.value = null;
}
</script>

<template>
  <div class="tags-page">
    <div class="tags-page__inner">
      <header class="tags-page__header">
        <div>
          <h1>{{ $t('tags.title') }}</h1>
          <p>{{ $t('tags.subtitle') }}</p>
        </div>
        <AppDialog :open="createDialogOpen" :title="$t('tags.createTitle')" @update:open="updateCreateDialog">
          <template #trigger>
            <AppButton>
              <Plus :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t('tags.create') }}</span>
            </AppButton>
          </template>
          <form class="tags-form" @submit.prevent="createTag">
            <label for="tag-create-name">{{ $t('tags.nameLabel') }}</label>
            <AppInput id="tag-create-name" v-model="createName" :placeholder="$t('tags.namePlaceholder')" :error="createError ? $t(`tags.errors.${createError}`) : ''" />
          </form>
          <template #footer>
            <AppButton variant="secondary" @click="createDialogOpen = false">{{ $t('tags.cancel') }}</AppButton>
            <AppButton :disabled="tagStore.workingId === 'create'" @click="createTag">{{ $t('tags.create') }}</AppButton>
          </template>
        </AppDialog>
      </header>

      <section v-if="tagStore.isLoading" class="tags-page__state" aria-busy="true">
        {{ $t('tags.loading') }}
      </section>
      <section v-else-if="tagStore.tags.length === 0" class="tags-page__state">
        <Tags :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t('tags.emptyTitle') }}</h2>
        <p>{{ $t('tags.emptyDescription') }}</p>
      </section>
      <ul v-else class="tags-page__list" :aria-label="$t('tags.existingLabel')">
        <li v-for="tag in tagStore.tags" :key="tag.id" class="tags-page__item">
          <RouterLink
            :to="{ name: 'tag-detail', params: { tagId: tag.id } }"
          >
            <Tag :size="16" :stroke-width="1.8" aria-hidden="true" />
            <span>{{ tag.name }}</span>
          </RouterLink>
          <div class="tags-page__actions">
            <AppDialog :open="renameCandidate?.id === tag.id" :title="$t('tags.renameTitle')" @update:open="updateRenameDialog(tag, $event)">
              <template #trigger><AppButton variant="ghost" :aria-label="$t('tags.rename')"><Pencil :size="17" :stroke-width="1.8" aria-hidden="true" /></AppButton></template>
              <form class="tags-form" @submit.prevent="renameTag">
                <label for="tag-rename-name">{{ $t('tags.nameLabel') }}</label>
                <AppInput id="tag-rename-name" v-model="renameName" :placeholder="$t('tags.namePlaceholder')" :error="renameError ? $t(`tags.errors.${renameError}`) : ''" />
              </form>
              <template #footer>
                <AppButton variant="secondary" @click="renameCandidate = null">{{ $t('tags.cancel') }}</AppButton>
                <AppButton :disabled="tagStore.workingId === tag.id" @click="renameTag">{{ $t('tags.save') }}</AppButton>
              </template>
            </AppDialog>
            <AppDialog :open="deleteCandidate?.id === tag.id" :title="$t('tags.deleteTitle')" :description="$t('tags.deleteDescription', { name: tag.name })" @update:open="deleteCandidate = $event ? tag : null">
              <template #trigger><AppButton variant="ghost" :aria-label="$t('tags.delete')"><Trash2 :size="17" :stroke-width="1.8" aria-hidden="true" /></AppButton></template>
              <template #footer>
                <AppButton variant="secondary" @click="deleteCandidate = null">{{ $t('tags.cancel') }}</AppButton>
                <AppButton variant="danger" :disabled="tagStore.workingId === tag.id" @click="deleteTag">{{ $t('tags.delete') }}</AppButton>
              </template>
            </AppDialog>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped lang="scss">
.tags-page {
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.tags-page__inner {
  display: grid;
  gap: var(--iv-space-6);
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.tags-page__header h1,
.tags-page__header p,
.tags-page__state h2,
.tags-page__state p {
  margin: 0;
}

.tags-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-6);
}

.tags-page__header h1 {
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.tags-page__header p,
.tags-page__state p {
  margin-top: var(--iv-space-1);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.tags-page__state {
  display: grid;
  justify-items: center;
  gap: var(--iv-space-2);
  min-height: 200px;
  padding: var(--iv-space-6);
  color: var(--iv-text-secondary);
  text-align: center;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.tags-page__state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  line-height: var(--iv-line-height-24);
}

.tags-page__list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--iv-space-3);
  padding: 0;
  margin: 0;
  list-style: none;
}

.tags-page__item {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-width: 0;
  padding: var(--iv-space-2);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.tags-page__item > a {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-width: 0;
  flex: 1;
  min-height: var(--iv-control-height);
  padding: 0 var(--iv-space-2);
  color: var(--iv-text-secondary);
  border-radius: var(--iv-radius-sm);
}

.tags-page__item > a:hover,
.tags-page__item > a:focus-visible {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent-soft);
  border-color: var(--iv-accent);
}

.tags-page__item > a:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.tags-page__actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--iv-space-1);
}

.tags-page__actions :deep(.app-button) {
  width: var(--iv-control-small-height);
  min-width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  padding: 0;
}

.tags-form {
  display: grid;
  gap: var(--iv-space-2);
}

@media (max-width: 640px) {
  .tags-page__header {
    flex-direction: column;
  }
}
</style>
