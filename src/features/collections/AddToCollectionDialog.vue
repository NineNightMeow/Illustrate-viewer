<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { FolderPlus } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import AppInput from "../../shared/components/ui/AppInput.vue";
import { useCollectionStore, type CollectionErrorCode } from "./collectionStore";

const props = defineProps<{ open: boolean; imageIds: string[] }>();
const emit = defineEmits<{ "update:open": [value: boolean]; added: [] }>();
const collectionStore = useCollectionStore();
const selectedIds = ref<string[]>([]);
const memberships = ref<Record<string, number>>({});
const newCollectionName = ref("");
const formError = ref<CollectionErrorCode | null>(null);
const isLoadingMemberships = ref(false);
const isSubmitting = computed(() => collectionStore.workingId !== null);
const uniqueImageIds = computed(() => [...new Set(props.imageIds)]);
const canSubmit = computed(() => uniqueImageIds.value.length > 0 && selectedIds.value.length > 0 && !isSubmitting.value);

function membershipCount(collectionId: string) { return memberships.value[collectionId] ?? 0; }
function membershipLabel(collectionId: string) {
  const count = membershipCount(collectionId);
  if (count === uniqueImageIds.value.length) return "collections.assignmentComplete";
  if (count > 0) return "collections.assignmentPartial";
  return "collections.assignmentNone";
}
function toggleCollection(collectionId: string) {
  selectedIds.value = selectedIds.value.includes(collectionId)
    ? selectedIds.value.filter((id) => id !== collectionId)
    : [...selectedIds.value, collectionId];
  formError.value = null;
}
async function loadMemberships() {
  isLoadingMemberships.value = true;
  const values = await collectionStore.loadMemberships(collectionStore.collections.map((collection) => collection.id), uniqueImageIds.value);
  memberships.value = Object.fromEntries(values.map((membership) => [membership.collectionId, membership.assignedCount]));
  formError.value = collectionStore.error;
  isLoadingMemberships.value = false;
}
watch(() => props.open, async (open) => {
  if (!open) return;
  selectedIds.value = [];
  memberships.value = {};
  newCollectionName.value = "";
  formError.value = null;
  await collectionStore.load();
  if (props.open && uniqueImageIds.value.length > 0) await loadMemberships();
});
async function addToCollections() {
  if (!canSubmit.value) {
    formError.value = uniqueImageIds.value.length === 0 ? "empty_image_selection" : "empty_collection_selection";
    return;
  }
  if (!await collectionStore.addImages(selectedIds.value, uniqueImageIds.value)) { formError.value = collectionStore.error; return; }
  emit("added"); emit("update:open", false);
}
async function createAndAdd() {
  if (uniqueImageIds.value.length === 0) { formError.value = "empty_image_selection"; return; }
  if (!await collectionStore.createAndAdd(newCollectionName.value, uniqueImageIds.value)) { formError.value = collectionStore.error; return; }
  emit("added"); emit("update:open", false);
}
</script>

<template>
  <AppDialog :open="open" :title="$t('collections.addImageTitle')" :description="$t('collections.addImageDescription')" @update:open="emit('update:open', $event)">
    <template #trigger><span aria-hidden="true" /></template>
    <section v-if="uniqueImageIds.length === 0" class="collection-picker__state" role="alert"><p>{{ $t("collections.emptyImageSelection") }}</p></section>
    <section v-else-if="collectionStore.isLoading || isLoadingMemberships" class="collection-picker__state" aria-busy="true">{{ $t("collections.loadingMemberships") }}</section>
    <div v-else class="collection-picker">
      <section v-if="collectionStore.collections.length === 0" class="collection-picker__state"><FolderPlus :size="22" :stroke-width="1.8" aria-hidden="true" /><p>{{ $t("collections.noCollectionToChoose") }}</p></section>
      <template v-else>
        <p class="collection-picker__label" id="collection-picker-label">{{ $t("collections.chooseCollections") }}</p>
        <div class="collection-picker__options" aria-labelledby="collection-picker-label">
          <button v-for="collection in collectionStore.collections" :key="collection.id" type="button" role="checkbox" :aria-checked="selectedIds.includes(collection.id)" :class="{ 'is-selected': selectedIds.includes(collection.id) }" @click="toggleCollection(collection.id)">
            <span class="collection-picker__name"><span class="collection-picker__check" aria-hidden="true">{{ selectedIds.includes(collection.id) ? "✓" : "" }}</span><span>{{ collection.name }}</span></span>
            <small>{{ $t(membershipLabel(collection.id), { count: membershipCount(collection.id), total: uniqueImageIds.length }) }}</small>
          </button>
        </div>
      </template>
      <div class="collection-picker__create">
        <AppInput v-model="newCollectionName" :placeholder="$t('collections.namePlaceholder')" :disabled="isSubmitting" />
        <AppButton variant="secondary" :disabled="!newCollectionName.trim() || isSubmitting" @click="createAndAdd"><FolderPlus :size="16" :stroke-width="1.8" aria-hidden="true" /><span>{{ $t("collections.createAndAdd") }}</span></AppButton>
      </div>
      <p v-if="formError" class="collection-picker__error" role="alert">{{ $t('collections.errors.' + formError) }}</p>
    </div>
    <template #footer>
      <AppButton variant="secondary" @click="emit('update:open', false)">{{ $t("collections.cancel") }}</AppButton>
      <AppButton :disabled="!canSubmit" @click="addToCollections">{{ $t("collections.addImage") }}</AppButton>
    </template>
  </AppDialog>
</template>

<style scoped lang="scss">
.collection-picker, .collection-picker__options { display: grid; gap: var(--iv-space-2); }
.collection-picker__label, .collection-picker__state p, .collection-picker__error { margin: 0; }
.collection-picker__label { color: var(--iv-text-secondary); font-size: var(--iv-font-size-13); line-height: var(--iv-line-height-18); }
.collection-picker__options { max-height: 240px; overflow: auto; }
.collection-picker__options button { display: flex; align-items: center; justify-content: space-between; gap: var(--iv-space-3); width: 100%; min-height: var(--iv-control-height); padding: 0 var(--iv-space-3); color: var(--iv-text-primary); text-align: left; background-color: var(--iv-bg-secondary); border: 1px solid var(--iv-border-subtle); border-radius: var(--iv-radius-sm); cursor: pointer; transition: border-color var(--iv-motion-fast) ease, background-color var(--iv-motion-fast) ease; }
.collection-picker__options button:hover, .collection-picker__options button.is-selected { background-color: var(--iv-accent-soft); border-color: var(--iv-accent); }
.collection-picker__options button:focus-visible { outline: var(--iv-focus-ring-width) solid var(--iv-accent); outline-offset: var(--iv-space-1); }
.collection-picker__name { display: inline-flex; align-items: center; gap: var(--iv-space-2); min-width: 0; }
.collection-picker__check { display: inline-grid; place-items: center; width: 18px; height: 18px; border: 1px solid var(--iv-border-subtle); border-radius: var(--iv-radius-sm); font-size: var(--iv-font-size-12); }
.collection-picker__options button.is-selected .collection-picker__check { color: var(--iv-text-primary); background-color: var(--iv-accent); border-color: var(--iv-accent); }
.collection-picker__options small { flex: 0 0 auto; color: var(--iv-text-secondary); font-size: var(--iv-font-size-12); }
.collection-picker__create { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: var(--iv-space-2); padding-top: var(--iv-space-2); }
.collection-picker__state { display: grid; justify-items: center; gap: var(--iv-space-3); min-height: 112px; color: var(--iv-text-secondary); text-align: center; }
.collection-picker__error { color: var(--iv-danger); font-size: var(--iv-font-size-13); line-height: var(--iv-line-height-18); }
</style>
