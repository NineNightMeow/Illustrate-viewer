<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { FileImage, Pencil, Sparkles, Trash2 } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import AppInput from "../../shared/components/ui/AppInput.vue";
import AppSelect, { type AppSelectOption } from "../../shared/components/ui/AppSelect.vue";
import {
  useSmartCollectionStore,
  type SmartCollectionErrorCode,
  type SmartCollectionSummary,
} from "./smartCollectionStore";
import type { SmartCollectionRule } from "./smartCollectionApi";

type RuleKind = SmartCollectionRule["kind"];

const { locale, t } = useI18n();
const smartCollectionStore = useSmartCollectionStore();
const createDialogOpen = ref(false);
const renameCandidate = ref<SmartCollectionSummary | null>(null);
const deleteCandidate = ref<SmartCollectionSummary | null>(null);
const createName = ref("");
const renameName = ref("");
const selectedRuleKind = ref<RuleKind>("largeImages");
const createError = ref<SmartCollectionErrorCode | null>(null);
const renameError = ref<SmartCollectionErrorCode | null>(null);
const dateFormatter = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }));
const ruleOptions = computed<AppSelectOption[]>(() => [
  { value: "largeImages", label: t("smartCollections.rules.largeImages") },
  { value: "minimumWidth", label: t("smartCollections.rules.minimumWidth") },
  { value: "minimumHeight", label: t("smartCollections.rules.minimumHeight") },
  { value: "format", label: t("smartCollections.rules.pngImages") },
  { value: "landscape", label: t("smartCollections.rules.landscape") },
  { value: "portrait", label: t("smartCollections.rules.portrait") },
  { value: "transparentPng", label: t("smartCollections.rules.transparentPng") },
  { value: "favorite", label: t("smartCollections.rules.favorite") },
]);

onMounted(() => void smartCollectionStore.load());

function formatDate(timestamp: number) {
  return dateFormatter.value.format(timestamp);
}

function ruleForKind(kind: RuleKind): SmartCollectionRule {
  if (kind === "format") return { kind, extension: "png" };
  if (kind === "minimumWidth") return { kind, minimumWidth: 1920 };
  if (kind === "minimumHeight") return { kind, minimumHeight: 1080 };
  if (kind === "landscape") return { kind };
  if (kind === "portrait") return { kind };
  if (kind === "transparentPng") return { kind };
  if (kind === "favorite") return { kind };
  return { kind, minimumPixels: 8_000_000 };
}

function ruleLabel(rule: SmartCollectionRule) {
  if (rule.kind === "format") return t("smartCollections.rules.pngImages");
  if (rule.kind === "minimumWidth") return t("smartCollections.rules.minimumWidth");
  if (rule.kind === "minimumHeight") return t("smartCollections.rules.minimumHeight");
  if (rule.kind === "landscape") return t("smartCollections.rules.landscape");
  if (rule.kind === "portrait") return t("smartCollections.rules.portrait");
  if (rule.kind === "transparentPng") return t("smartCollections.rules.transparentPng");
  if (rule.kind === "favorite") return t("smartCollections.rules.favorite");
  return t("smartCollections.rules.largeImages");
}

function updateCreateDialog(open: boolean) {
  createDialogOpen.value = open;
  if (open) {
    createName.value = "";
    selectedRuleKind.value = "largeImages";
    createError.value = null;
  }
}

function updateRenameDialog(collection: SmartCollectionSummary, open: boolean) {
  renameCandidate.value = open ? collection : null;
  if (open) {
    renameName.value = collection.name;
    renameError.value = null;
  }
}

async function createCollection() {
  const collection = await smartCollectionStore.create(createName.value, ruleForKind(selectedRuleKind.value));
  if (collection) {
    createDialogOpen.value = false;
    return;
  }
  createError.value = smartCollectionStore.error;
}

async function renameCollection() {
  if (!renameCandidate.value) return;
  const collection = await smartCollectionStore.rename(renameCandidate.value.id, renameName.value);
  if (collection) {
    renameCandidate.value = null;
    return;
  }
  renameError.value = smartCollectionStore.error;
}

async function deleteCollection() {
  if (!deleteCandidate.value) return;
  if (await smartCollectionStore.remove(deleteCandidate.value.id)) deleteCandidate.value = null;
}
</script>

<template>
  <div class="smart-collections-page">
    <div class="smart-collections-page__inner">
      <header class="smart-collections-page__header">
        <div>
          <h1>{{ $t("smartCollections.title") }}</h1>
          <p>{{ $t("smartCollections.subtitle") }}</p>
        </div>
        <AppDialog
          :open="createDialogOpen"
          :title="$t('smartCollections.createTitle')"
          :description="$t('smartCollections.createDescription')"
          @update:open="updateCreateDialog"
        >
          <template #trigger>
            <AppButton>
              <Sparkles :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t("smartCollections.create") }}</span>
            </AppButton>
          </template>
          <form class="smart-collections-form" @submit.prevent="createCollection">
            <label for="smart-collection-create-name">{{ $t("smartCollections.nameLabel") }}</label>
            <AppInput
              id="smart-collection-create-name"
              v-model="createName"
              :placeholder="$t('smartCollections.namePlaceholder')"
              :error="createError ? $t(`smartCollections.errors.${createError}`) : ''"
              :aria-describedby="createError ? 'smart-collection-create-error' : undefined"
              @blur="createError = null"
            />
            <label>{{ $t("smartCollections.ruleLabel") }}</label>
            <AppSelect
              v-model="selectedRuleKind"
              :options="ruleOptions"
              :aria-label="$t('smartCollections.ruleLabel')"
            />
            <p v-if="createError" id="smart-collection-create-error" role="alert">
              {{ $t(`smartCollections.errors.${createError}`) }}
            </p>
          </form>
          <template #footer>
            <AppButton variant="secondary" @click="createDialogOpen = false">
              {{ $t("smartCollections.cancel") }}
            </AppButton>
            <AppButton :disabled="smartCollectionStore.workingId === 'create'" @click="createCollection">
              {{ smartCollectionStore.workingId === "create" ? $t("smartCollections.creating") : $t("smartCollections.create") }}
            </AppButton>
          </template>
        </AppDialog>
      </header>

      <p v-if="smartCollectionStore.error && !createError && !renameError" class="smart-collections-alert" role="alert">
        {{ $t(`smartCollections.errors.${smartCollectionStore.error}`) }}
      </p>

      <section v-if="smartCollectionStore.isLoading" class="smart-collections-state" aria-busy="true">
        {{ $t("smartCollections.loading") }}
      </section>

      <section v-else-if="smartCollectionStore.collections.length === 0" class="smart-collections-state">
        <Sparkles :size="28" :stroke-width="1.6" aria-hidden="true" />
        <h2>{{ $t("smartCollections.emptyTitle") }}</h2>
        <p>{{ $t("smartCollections.emptyDescription") }}</p>
        <AppButton @click="createDialogOpen = true">
          <Sparkles :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t("smartCollections.create") }}</span>
        </AppButton>
      </section>

      <ul v-else class="smart-collections-list" :aria-label="$t('smartCollections.title')">
        <li v-for="collection in smartCollectionStore.collections" :key="collection.id" class="smart-collection-card" v-memo="[collection.id, collection.name, collection.imageCount, collection.updatedAt]">
          <RouterLink class="smart-collection-card__open" :to="{ name: 'smart-collection-detail', params: { collectionId: collection.id } }">
            <span class="smart-collection-card__icon" aria-hidden="true"><FileImage :size="20" :stroke-width="1.8" /></span>
            <span class="smart-collection-card__copy">
              <strong>{{ collection.name }}</strong>
              <small>{{ ruleLabel(collection.rule) }}</small>
              <small>{{ $t("smartCollections.count", { count: collection.imageCount }) }}</small>
              <small>{{ $t("smartCollections.created", { date: formatDate(collection.createdAt) }) }}</small>
            </span>
          </RouterLink>
          <div class="smart-collection-card__actions">
            <AppDialog
              :open="renameCandidate?.id === collection.id"
              :title="$t('smartCollections.renameTitle')"
              @update:open="updateRenameDialog(collection, $event)"
            >
              <template #trigger>
                <AppButton variant="ghost" :aria-label="$t('smartCollections.rename')">
                  <Pencil :size="17" :stroke-width="1.8" aria-hidden="true" />
                </AppButton>
              </template>
              <form class="smart-collections-form" @submit.prevent="renameCollection">
                <label for="smart-collection-rename-name">{{ $t("smartCollections.nameLabel") }}</label>
                <AppInput
                  id="smart-collection-rename-name"
                  v-model="renameName"
                  :placeholder="$t('smartCollections.namePlaceholder')"
                  :error="renameError ? $t(`smartCollections.errors.${renameError}`) : ''"
                  :aria-describedby="renameError ? 'smart-collection-rename-error' : undefined"
                  @blur="renameError = null"
                />
                <p v-if="renameError" id="smart-collection-rename-error" role="alert">
                  {{ $t(`smartCollections.errors.${renameError}`) }}
                </p>
              </form>
              <template #footer>
                <AppButton variant="secondary" @click="renameCandidate = null">
                  {{ $t("smartCollections.cancel") }}
                </AppButton>
                <AppButton :disabled="smartCollectionStore.workingId === collection.id" @click="renameCollection">
                  {{ smartCollectionStore.workingId === collection.id ? $t("smartCollections.saving") : $t("smartCollections.save") }}
                </AppButton>
              </template>
            </AppDialog>

            <AppDialog
              :open="deleteCandidate?.id === collection.id"
              :title="$t('smartCollections.deleteTitle')"
              :description="$t('smartCollections.deleteDescription', { name: collection.name })"
              @update:open="deleteCandidate = $event ? collection : null"
            >
              <template #trigger>
                <AppButton variant="ghost" :aria-label="$t('smartCollections.delete')">
                  <Trash2 :size="17" :stroke-width="1.8" aria-hidden="true" />
                </AppButton>
              </template>
              <template #footer>
                <AppButton variant="secondary" @click="deleteCandidate = null">
                  {{ $t("smartCollections.cancel") }}
                </AppButton>
                <AppButton variant="danger" :disabled="smartCollectionStore.workingId === collection.id" @click="deleteCollection">
                  {{ smartCollectionStore.workingId === collection.id ? $t("smartCollections.deleting") : $t("smartCollections.delete") }}
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
.smart-collections-page { min-height: 100%; padding: var(--iv-content-padding); }
.smart-collections-page__inner { display: grid; gap: var(--iv-space-6); width: 100%; max-width: var(--iv-content-max-width); margin: 0 auto; }
.smart-collections-page__header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--iv-space-6); }
.smart-collections-page__header h1, .smart-collections-page__header p, .smart-collections-state h2, .smart-collections-state p, .smart-collections-form p { margin: 0; }
.smart-collections-page__header h1 { font-size: var(--iv-font-size-24); font-weight: var(--iv-font-weight-semibold); line-height: var(--iv-line-height-32); }
.smart-collections-page__header p, .smart-collections-state p, .smart-collection-card small { color: var(--iv-text-secondary); font-size: var(--iv-font-size-14); line-height: var(--iv-line-height-20); }
.smart-collections-page__header p { margin-top: var(--iv-space-1); }
.smart-collections-alert { margin: 0; padding: var(--iv-space-3) var(--iv-space-4); color: var(--iv-danger); background-color: var(--iv-danger-soft); border: 1px solid var(--iv-danger); border-radius: var(--iv-radius-sm); }
.smart-collections-state { display: grid; justify-items: center; gap: var(--iv-space-3); min-height: 260px; padding: var(--iv-space-8); color: var(--iv-text-secondary); text-align: center; background-color: var(--iv-surface); border: 1px solid var(--iv-border-subtle); border-radius: var(--iv-radius-md); }
.smart-collections-state h2 { color: var(--iv-text-primary); font-size: var(--iv-font-size-16); font-weight: var(--iv-font-weight-semibold); line-height: var(--iv-line-height-22); }
.smart-collections-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(264px, 1fr)); gap: var(--iv-space-3); padding: 0; margin: 0; list-style: none; }
.smart-collection-card { display: flex; align-items: center; gap: var(--iv-space-2); min-width: 0; padding: var(--iv-space-3); background-color: var(--iv-surface); border: 1px solid var(--iv-border-subtle); border-radius: var(--iv-radius-md); transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease; }
.smart-collection-card:hover { background-color: var(--iv-surface-hover); border-color: var(--iv-accent); }
.smart-collection-card__open { display: flex; align-items: center; gap: var(--iv-space-3); flex: 1; min-width: 0; color: inherit; }
.smart-collection-card__open:focus-visible { outline: var(--iv-focus-ring-width) solid var(--iv-accent); outline-offset: var(--iv-space-1); }
.smart-collection-card__icon { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 var(--iv-control-height); width: var(--iv-control-height); height: var(--iv-control-height); color: var(--iv-accent); background-color: var(--iv-accent-soft); border-radius: var(--iv-radius-sm); }
.smart-collection-card__copy { display: grid; min-width: 0; }
.smart-collection-card__copy strong, .smart-collection-card__copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.smart-collection-card__copy strong { font-size: var(--iv-font-size-15); font-weight: var(--iv-font-weight-semibold); line-height: var(--iv-line-height-20); }
.smart-collection-card__copy small { font-size: var(--iv-font-size-12); line-height: var(--iv-line-height-16); }
.smart-collection-card__actions { display: flex; flex: 0 0 auto; gap: var(--iv-space-1); }
.smart-collection-card__actions :deep(.app-button) { width: var(--iv-control-small-height); min-width: var(--iv-control-small-height); height: var(--iv-control-small-height); padding: 0; }
.smart-collections-form { display: grid; gap: var(--iv-space-2); }
.smart-collections-form label { color: var(--iv-text-secondary); font-size: var(--iv-font-size-13); line-height: var(--iv-line-height-18); }
.smart-collections-form :deep(.app-select) { width: 100%; }
.smart-collections-form p { color: var(--iv-danger); font-size: var(--iv-font-size-13); line-height: var(--iv-line-height-18); }
@media (max-width: 640px) { .smart-collections-page__header { flex-direction: column; } }
</style>
