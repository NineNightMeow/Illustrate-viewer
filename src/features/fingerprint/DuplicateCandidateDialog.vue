<script setup lang="ts">
import { computed } from "vue";
import { Files, LoaderCircle } from "lucide-vue-next";
import AppButton from "../../shared/components/ui/AppButton.vue";
import AppDialog from "../../shared/components/ui/AppDialog.vue";
import ThumbnailPreview from "../library/components/ThumbnailPreview.vue";
import type { ImageAsset, ThumbnailRecord } from "../library/types";
import type { DuplicateCandidate } from "./fingerprintApi";

const props = defineProps<{
  open: boolean;
  source: ImageAsset | null;
  candidates: DuplicateCandidate[];
  totalCount: number;
  isLoading: boolean;
  hasError: boolean;
  recordsByImageId: Record<string, ThumbnailRecord>;
  generatingLabel: string;
  unavailableLabel: string;
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  "load-more": [];
}>();

const hasMore = computed(() => props.candidates.length < props.totalCount);

function similarity(distance: number) {
  return Math.max(0, Math.round((1 - distance / 64) * 100));
}
</script>

<template>
  <AppDialog
    :open="open"
    :title="$t('duplicates.title')"
    :description="$t('duplicates.description')"
    @update:open="emit('update:open', $event)"
  >
    <template #trigger><span aria-hidden="true" /></template>

    <section v-if="source" class="duplicate-candidates__source" :aria-label="$t('duplicates.currentImage')">
      <ThumbnailPreview
        class="duplicate-candidates__thumbnail"
        :record="recordsByImageId[source.id] ?? null"
        :label="source.filename"
        :generating-label="generatingLabel"
        :unavailable-label="unavailableLabel"
      />
      <div>
        <span>{{ $t('duplicates.currentImage') }}</span>
        <strong :title="source.filename">{{ source.filename }}</strong>
      </div>
    </section>

    <section v-if="isLoading && candidates.length === 0" class="duplicate-candidates__state" aria-busy="true">
      <LoaderCircle class="duplicate-candidates__spinner" :size="22" :stroke-width="1.8" aria-hidden="true" />
      <p>{{ $t('duplicates.loading') }}</p>
    </section>
    <section v-else-if="hasError" class="duplicate-candidates__state duplicate-candidates__state--error" role="status">
      <p>{{ $t('duplicates.unavailable') }}</p>
    </section>
    <section v-else-if="candidates.length === 0" class="duplicate-candidates__state">
      <Files :size="24" :stroke-width="1.7" aria-hidden="true" />
      <p>{{ $t('duplicates.empty') }}</p>
    </section>
    <ol v-else class="duplicate-candidates__list">
      <li v-for="candidate in candidates" :key="candidate.asset.id" v-memo="[candidate.asset.id, candidate.matchKind, candidate.perceptualDistance]">
        <ThumbnailPreview
          class="duplicate-candidates__thumbnail"
          :record="recordsByImageId[candidate.asset.id] ?? null"
          :label="candidate.asset.filename"
          :generating-label="generatingLabel"
          :unavailable-label="unavailableLabel"
        />
        <div class="duplicate-candidates__details">
          <strong :title="candidate.asset.filename">{{ candidate.asset.filename }}</strong>
          <span v-if="candidate.matchKind === 'exact'">{{ $t('duplicates.exactMatch') }}</span>
          <span v-else>{{ $t('duplicates.similarity', { value: similarity(candidate.perceptualDistance) }) }}</span>
          <small>{{ $t('duplicates.hashDistance', { distance: candidate.perceptualDistance }) }}</small>
        </div>
      </li>
    </ol>

    <template #footer>
      <AppButton v-if="hasMore" variant="secondary" :disabled="isLoading" @click="emit('load-more')">
        <LoaderCircle v-if="isLoading" class="duplicate-candidates__spinner" :size="16" :stroke-width="1.8" aria-hidden="true" />
        <span>{{ $t('duplicates.loadMore') }}</span>
      </AppButton>
      <AppButton variant="secondary" @click="emit('update:open', false)">
        {{ $t('duplicates.close') }}
      </AppButton>
    </template>
  </AppDialog>
</template>

<style scoped lang="scss">
.duplicate-candidates__source,
.duplicate-candidates__list li {
  display: grid;
  grid-template-columns: 56px minmax(0, 1fr);
  gap: var(--iv-space-3);
  align-items: center;
}

.duplicate-candidates__source {
  margin-bottom: var(--iv-space-4);
  padding: var(--iv-space-3);
  background-color: var(--iv-bg-secondary);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
}

.duplicate-candidates__source div,
.duplicate-candidates__details {
  display: grid;
  min-width: 0;
  gap: var(--iv-space-1);
}

.duplicate-candidates__source span,
.duplicate-candidates__details span,
.duplicate-candidates__details small {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.duplicate-candidates__source strong,
.duplicate-candidates__details strong {
  overflow: hidden;
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-13);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-18);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.duplicate-candidates__thumbnail {
  width: 56px;
  height: 56px;
  overflow: hidden;
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-xs);
}

.duplicate-candidates__list {
  display: grid;
  gap: var(--iv-space-2);
  max-height: min(54vh, 520px);
  margin: 0;
  padding: 0;
  overflow: auto;
  list-style: none;
}

.duplicate-candidates__list li {
  padding: var(--iv-space-2);
  background-color: var(--iv-bg-secondary);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-sm);
}

.duplicate-candidates__details span {
  color: var(--iv-accent-hover);
}

.duplicate-candidates__details small {
  font-variant-numeric: tabular-nums;
}

.duplicate-candidates__state {
  display: grid;
  justify-items: center;
  gap: var(--iv-space-3);
  min-height: 160px;
  margin: 0;
  color: var(--iv-text-secondary);
  text-align: center;
}

.duplicate-candidates__state p {
  margin: 0;
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.duplicate-candidates__state--error {
  color: var(--iv-danger);
}

.duplicate-candidates__spinner {
  animation: duplicate-candidates-spin 800ms linear infinite;
}

@keyframes duplicate-candidates-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .duplicate-candidates__spinner {
    animation: none;
  }
}
</style>
