<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { FolderPlus, Images, Pencil, Trash2 } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import AppInput from "../../shared/components/ui/AppInput.vue";
import { useCollectionStore, type CollectionErrorCode, type CollectionSummary } from "./collectionStore";

const { locale, t } = useI18n();
const collectionStore = useCollectionStore();
const createDialogOpen = ref(false);
const renameCandidate = ref<CollectionSummary | null>(null);
const deleteCandidate = ref<CollectionSummary | null>(null);
const createName = ref("");
const renameName = ref("");
const createError = ref<CollectionErrorCode | null>(null);
const renameError = ref<CollectionErrorCode | null>(null);
const dateFormatter = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }));
const pageError = computed(() => collectionStore.error ? t(`collections.errors.${collectionStore.error}`) : null);

onMounted(() => {
  void collectionStore.load();
});

function formatDate(timestamp: number) {
  return dateFormatter.value.format(timestamp);
}

function updateCreateDialog(open: boolean) {
  createDialogOpen.value = open;
  if (open) {
    createName.value = "";
    createError.value = null;
  }
}

function updateRenameDialog(collection: CollectionSummary, open: boolean) {
  renameCandidate.value = open ? collection : null;
  if (open) {
    renameName.value = collection.name;
    renameError.value = null;
  }
}

async function createCollection() {
  const collection = await collectionStore.create(createName.value);
  if (collection) {
    createDialogOpen.value = false;
    return;
  }
  createError.value = collectionStore.error;
}

async function renameCollection() {
  if (!renameCandidate.value) return;
  const collection = await collectionStore.rename(renameCandidate.value.id, renameName.value);
  if (collection) {
    renameCandidate.value = null;
    return;
  }
  renameError.value = collectionStore.error;
}

async function deleteCollection() {
  if (!deleteCandidate.value) return;
  if (await collectionStore.remove(deleteCandidate.value.id)) deleteCandidate.value = null;
}
</script>

<template>
  <div class="collections-page">
    <div class="collections-page__inner">
      <header class="collections-page__header">
        <div>
          <h1>{{ $t("collections.title") }}</h1>
          <p>{{ $t("collections.subtitle") }}</p>
        </div>
        <AppDialog
          :open="createDialogOpen"
          :title="$t('collections.createTitle')"
          @update:open="updateCreateDialog"
        >
          <template #trigger>
            <AppButton>
              <FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t("collections.create") }}</span>
            </AppButton>
          </template>
          <form class="collections-form" @submit.prevent="createCollection">
            <label for="collection-create-name">{{ $t("collections.nameLabel") }}</label>
            <AppInput
              id="collection-create-name"
              v-model="createName"
              :placeholder="$t('collections.namePlaceholder')"
              :error="createError ? $t(`collections.errors.${createError}`) : ''"
              :aria-describedby="createError ? 'collection-create-error' : undefined"
              @blur="createError = null"
            />
            <p v-if="createError" id="collection-create-error" role="alert">
              {{ $t(`collections.errors.${createError}`) }}
            </p>
          </form>
          <template #footer>
            <AppButton variant="secondary" @click="createDialogOpen = false">
              {{ $t("collections.cancel") }}
            </AppButton>
            <AppButton :disabled="collectionStore.workingId === 'create'" @click="createCollection">
              {{ collectionStore.workingId === "create" ? $t("collections.creating") : $t("collections.create") }}
            </AppButton>
          </template>
        </AppDialog>
      </header>

      <p v-if="pageError && !createError && !renameError" class="collections-alert" role="alert">
        {{ pageError }}
      </p>

      <section v-if="collectionStore.isLoading" class="collections-state" aria-busy="true">
        {{ $t("collections.loading") }}
      </section>

      <section v-else-if="collectionStore.collections.length === 0" class="collections-state">
        <FolderPlus :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("collections.emptyTitle") }}</h2>
        <p>{{ $t("collections.emptyDescription") }}</p>
        <AppButton @click="createDialogOpen = true">
          <FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("collections.create") }}</span>
        </AppButton>
      </section>

      <ul v-else class="collections-list" :aria-label="$t('collections.title')">
        <li v-for="collection in collectionStore.collections" :key="collection.id" class="collection-card">
          <RouterLink class="collection-card__open" :to="{ name: 'collection-detail', params: { collectionId: collection.id } }">
            <span class="collection-card__icon" aria-hidden="true"><Images :size="20" :stroke-width="1.8" /></span>
            <span class="collection-card__copy">
              <strong>{{ collection.name }}</strong>
              <small>{{ $t("collections.count", { count: collection.imageCount }) }}</small>
              <small>{{ $t("collections.created", { date: formatDate(collection.createdAt) }) }}</small>
            </span>
          </RouterLink>
          <div class="collection-card__actions">
            <AppDialog
              :open="renameCandidate?.id === collection.id"
              :title="$t('collections.renameTitle')"
              @update:open="updateRenameDialog(collection, $event)"
            >
              <template #trigger>
                <AppButton variant="ghost" :aria-label="$t('collections.rename')">
                  <Pencil :size="17" :stroke-width="1.8" aria-hidden="true" />
                </AppButton>
              </template>
              <form class="collections-form" @submit.prevent="renameCollection">
                <label for="collection-rename-name">{{ $t("collections.nameLabel") }}</label>
                <AppInput
                  id="collection-rename-name"
                  v-model="renameName"
                  :placeholder="$t('collections.namePlaceholder')"
                  :error="renameError ? $t(`collections.errors.${renameError}`) : ''"
                  :aria-describedby="renameError ? 'collection-rename-error' : undefined"
                  @blur="renameError = null"
                />
                <p v-if="renameError" id="collection-rename-error" role="alert">
                  {{ $t(`collections.errors.${renameError}`) }}
                </p>
              </form>
              <template #footer>
                <AppButton variant="secondary" @click="renameCandidate = null">
                  {{ $t("collections.cancel") }}
                </AppButton>
                <AppButton :disabled="collectionStore.workingId === collection.id" @click="renameCollection">
                  {{ collectionStore.workingId === collection.id ? $t("collections.saving") : $t("collections.save") }}
                </AppButton>
              </template>
            </AppDialog>

            <AppDialog
              :open="deleteCandidate?.id === collection.id"
              :title="$t('collections.deleteTitle')"
              :description="$t('collections.deleteDescription', { name: collection.name })"
              @update:open="deleteCandidate = $event ? collection : null"
            >
              <template #trigger>
                <AppButton variant="ghost" :aria-label="$t('collections.delete')">
                  <Trash2 :size="17" :stroke-width="1.8" aria-hidden="true" />
                </AppButton>
              </template>
              <template #footer>
                <AppButton variant="secondary" @click="deleteCandidate = null">
                  {{ $t("collections.cancel") }}
                </AppButton>
                <AppButton variant="danger" :disabled="collectionStore.workingId === collection.id" @click="deleteCollection">
                  {{ collectionStore.workingId === collection.id ? $t("collections.deleting") : $t("collections.delete") }}
                </AppButton>
              </template>
            </AppDialog>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped lang="scss">
.collections-page {
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.collections-page__inner {
  display: grid;
  gap: var(--iv-space-6);
  width: 100%;
  max-width: var(--iv-content-max-width);
  margin: 0 auto;
}

.collections-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-6);
}

.collections-page__header h1,
.collections-page__header p,
.collections-state h2,
.collections-state p,
.collections-form p {
  margin: 0;
}

.collections-page__header h1 {
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.collections-page__header p,
.collections-state p,
.collection-card small {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.collections-page__header p {
  margin-top: var(--iv-space-1);
}

.collections-alert {
  margin: 0;
  padding: var(--iv-space-3) var(--iv-space-4);
  color: var(--iv-danger);
  background-color: var(--iv-danger-soft);
  border: 1px solid var(--iv-danger);
  border-radius: var(--iv-radius-sm);
}

.collections-state {
  display: grid;
  justify-items: center;
  gap: var(--iv-space-3);
  min-height: 260px;
  padding: var(--iv-space-8);
  color: var(--iv-text-secondary);
  text-align: center;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.collections-state h2 {
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-22);
}

.collections-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(264px, 1fr));
  gap: var(--iv-space-3);
  padding: 0;
  margin: 0;
  list-style: none;
}

.collection-card {
  display: flex;
  align-items: center;
  gap: var(--iv-space-2);
  min-width: 0;
  padding: var(--iv-space-3);
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
  transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease;
}

.collection-card:hover {
  background-color: var(--iv-surface-hover);
  border-color: var(--iv-accent);
}

.collection-card__open {
  display: flex;
  align-items: center;
  gap: var(--iv-space-3);
  flex: 1;
  min-width: 0;
  color: inherit;
}

.collection-card__open:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.collection-card__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 var(--iv-control-height);
  width: var(--iv-control-height);
  height: var(--iv-control-height);
  color: var(--iv-accent);
  background-color: var(--iv-accent-soft);
  border-radius: var(--iv-radius-sm);
}

.collection-card__copy {
  display: grid;
  min-width: 0;
}

.collection-card__copy strong,
.collection-card__copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.collection-card__copy strong {
  font-size: var(--iv-font-size-15);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-20);
}

.collection-card__copy small {
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.collection-card__actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--iv-space-1);
}

.collection-card__actions :deep(.app-button) {
  width: var(--iv-control-small-height);
  min-width: var(--iv-control-small-height);
  height: var(--iv-control-small-height);
  padding: 0;
}

.collections-form {
  display: grid;
  gap: var(--iv-space-2);
}

.collections-form label {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.collections-form p {
  color: var(--iv-danger);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

@media (max-width: 640px) {
  .collections-page__header {
    flex-direction: column;
  }
}
</style>
