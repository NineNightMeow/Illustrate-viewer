<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Check, Plus, Tags } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import AppInput from "../../shared/components/ui/AppInput.vue";
import { useTagStore, type TagErrorCode } from "./tagStore";

const props = defineProps<{
  open: boolean;
  imageIds: string[];
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  updated: [];
}>();

const tagStore = useTagStore();
const assignedIds = ref(new Set<string>());
const newTagName = ref("");
const formError = ref<TagErrorCode | null>(null);
const selectedImageIds = computed(() => [...new Set(props.imageIds)]);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    newTagName.value = "";
    formError.value = null;
    void loadDialog();
  },
);

async function loadDialog() {
  await tagStore.load();
  const assignmentsByImageId = await tagStore.loadImageTagsForImages(selectedImageIds.value);
  const counts = new Map<string, number>();
  for (const imageId of selectedImageIds.value) {
    const tags = assignmentsByImageId[imageId] ?? [];
    for (const tag of tags) counts.set(tag.id, (counts.get(tag.id) ?? 0) + 1);
  }
  assignedIds.value = new Set(
    [...counts].filter(([, count]) => count === selectedImageIds.value.length).map(([tagId]) => tagId),
  );
}

async function toggleTag(tagId: string) {
  if (selectedImageIds.value.length === 0) return;

  const wasAssigned = assignedIds.value.has(tagId);
  const completed = wasAssigned
    ? await tagStore.removeFromImages(tagId, selectedImageIds.value)
    : await tagStore.addToImages(tagId, selectedImageIds.value);
  if (!completed) {
    formError.value = tagStore.error;
    return;
  }

  assignedIds.value = new Set(assignedIds.value);
  if (wasAssigned) assignedIds.value.delete(tagId);
  else assignedIds.value.add(tagId);
  formError.value = null;
  emit("updated");
}

async function createAndAssign() {
  const tag = await tagStore.create(newTagName.value);
  if (!tag) {
    formError.value = tagStore.error;
    return;
  }

  newTagName.value = "";
  await toggleTag(tag.id);
}
</script>

<template>
  <AppDialog
    :open="open"
    :title="$t('tags.assignTitle')"
    :description="$t('tags.assignDescription', { count: selectedImageIds.length })"
    @update:open="emit('update:open', $event)"
  >
    <template #trigger><span aria-hidden="true" /></template>

    <section v-if="tagStore.isLoading" class="tag-picker__state" aria-busy="true">
      {{ $t('tags.loading') }}
    </section>
    <div v-else class="tag-picker">
      <div class="tag-picker__create">
        <AppInput
          v-model="newTagName"
          :placeholder="$t('tags.namePlaceholder')"
          :aria-label="$t('tags.nameLabel')"
          @keyup.enter="createAndAssign"
        />
        <AppButton :disabled="!newTagName.trim() || tagStore.workingId === 'create'" @click="createAndAssign">
          <Plus :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t('tags.create') }}</span>
        </AppButton>
      </div>

      <p v-if="tagStore.tags.length === 0" class="tag-picker__state">
        <Tags :size="22" :stroke-width="1.8" aria-hidden="true" />
        <span>{{ $t('tags.emptyTitle') }}</span>
      </p>
      <div v-else class="tag-picker__options" role="list" :aria-label="$t('tags.existingLabel')">
        <button
          v-for="tag in tagStore.tags"
          :key="tag.id"
          type="button"
          role="listitem"
          :class="{ 'is-selected': assignedIds.has(tag.id) }"
          :aria-pressed="assignedIds.has(tag.id)"
          :disabled="tagStore.workingId === tag.id"
          @click="toggleTag(tag.id)"
        >
          <span>{{ tag.name }}</span>
          <Check v-if="assignedIds.has(tag.id)" :size="16" :stroke-width="2" aria-hidden="true" />
        </button>
      </div>

      <p v-if="formError" class="tag-picker__error" role="alert">
        {{ $t(`tags.errors.${formError}`) }}
      </p>
    </div>

    <template #footer>
      <AppButton variant="secondary" @click="emit('update:open', false)">
        {{ $t('tags.done') }}
      </AppButton>
    </template>
  </AppDialog>
</template>

<style scoped lang="scss">
.tag-picker {
  display: grid;
  gap: var(--iv-space-3);
}

.tag-picker__create {
  display: flex;
  gap: var(--iv-space-2);
}

.tag-picker__create .app-button {
  flex: 0 0 auto;
}

.tag-picker__options {
  display: flex;
  flex-wrap: wrap;
  gap: var(--iv-space-2);
  max-height: 224px;
  overflow: auto;
}

.tag-picker__options button {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-1);
  min-height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-text-secondary);
  background-color: var(--iv-bg-secondary);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-full);
  cursor: pointer;
}

.tag-picker__options button:hover:not(:disabled),
.tag-picker__options button.is-selected {
  color: var(--iv-text-primary);
  background-color: var(--iv-accent-soft);
  border-color: var(--iv-accent);
}

.tag-picker__options button:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.tag-picker__options button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.tag-picker__state,
.tag-picker__error {
  margin: 0;
}

.tag-picker__state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--iv-space-2);
  min-height: 88px;
  color: var(--iv-text-secondary);
  text-align: center;
}

.tag-picker__error {
  color: var(--iv-danger);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}
</style>
