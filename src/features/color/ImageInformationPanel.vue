<script setup lang="ts">
import { computed } from "vue";
import { LoaderCircle, Pipette, RefreshCw, Tags, X } from "lucide-vue-next";
import type { Color, ImageColorMetadata } from "./colorApi";
import type { DuplicateCandidatePage } from "../fingerprint/fingerprintApi";
import type { ImageMetadata, MetadataTaskStatus } from "../metadata/metadataApi";

type PickedColor = {
  color: Color;
  x: number;
  y: number;
};

const props = defineProps<{
  open: boolean;
  filename: string | null;
  imageMetadata: ImageMetadata | null;
  metadataTaskStatus: MetadataTaskStatus;
  hasMetadataError: boolean;
  canLoadMetadata: boolean;
  metadata: ImageColorMetadata | null;
  selectedColor: Color | null;
  pickedColor: PickedColor | null;
  isAnalyzing: boolean;
  hasError: boolean;
  pickerActive: boolean;
  canAnalyze: boolean;
  canPick: boolean;
  duplicateSummary: DuplicateCandidatePage | null;
  isDuplicatesLoading: boolean;
  hasDuplicatesError: boolean;
  canViewDuplicates: boolean;
  tags: { id: string; name: string }[];
}>();

const emit = defineEmits<{
  close: [];
  analyze: [];
  "toggle-picker": [];
  "select-color": [color: Color];
  "open-duplicates": [];
  "retry-metadata": [];
  "manage-tags": [];
  "remove-tag": [tagId: string];
}>();

const detailColor = computed(() => props.selectedColor ?? props.pickedColor?.color ?? null);
const wheel = computed(() => toHsv(detailColor.value));
const hueMarkerStyle = computed(() => ({
  left: `${50 + Math.cos((wheel.value.hue - 90) * Math.PI / 180) * 39}%`,
  top: `${50 + Math.sin((wheel.value.hue - 90) * Math.PI / 180) * 39}%`,
}));
const saturationMarkerStyle = computed(() => ({
  left: `${wheel.value.saturation}%`,
  top: `${100 - wheel.value.value}%`,
}));
const saturationValueStyle = computed(() => ({
  "--iv-color-wheel-hue": `${wheel.value.hue}`,
}));

function toHsv(color: Color | null) {
  const channels = color?.rgb.split(",").map((channel) => Number(channel.trim())) ?? [];
  const [red = 0, green = 0, blue = 0] = channels.map((channel) => Math.max(0, Math.min(255, channel)) / 255);
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const delta = maximum - minimum;
  const hue = delta === 0 ? 0 : maximum === red
    ? 60 * positiveModulo((green - blue) / delta, 6)
    : maximum === green
      ? 60 * ((blue - red) / delta + 2)
      : 60 * ((red - green) / delta + 4);

  return {
    hue: Math.round(hue) % 360,
    saturation: maximum === 0 ? 0 : Math.round((delta / maximum) * 100),
    value: Math.round(maximum * 100),
  };
}

function positiveModulo(value: number, divisor: number) {
  return ((value % divisor) + divisor) % divisor;
}

function formatAspectRatio(width: number | null, height: number | null) {
  if (!width || !height) return "—";
  let left = width;
  let right = height;
  while (right !== 0) [left, right] = [right, left % right];
  return `${width / left}:${height / left}`;
}
</script>

<template>
  <aside
    v-if="open"
    class="image-information iv-glass-surface"
    :aria-label="$t('viewer.imageInformation')"
  >
    <header class="image-information__header">
      <div>
        <p class="image-information__eyebrow">{{ $t('viewer.imageInformation') }}</p>
        <h2 :title="filename ?? undefined">{{ filename ?? $t('viewer.title') }}</h2>
      </div>
      <button type="button" :aria-label="$t('viewer.closeInformation')" @click="emit('close')">
        <X :size="18" :stroke-width="1.8" aria-hidden="true" />
      </button>
    </header>

    <div class="image-information__scroll-content">
      <div class="image-information__actions">
      <button
        type="button"
        :disabled="!canAnalyze || isAnalyzing"
        @click="emit('analyze')"
      >
        <LoaderCircle v-if="isAnalyzing" class="image-information__spinner" :size="16" :stroke-width="1.8" aria-hidden="true" />
        <RefreshCw v-else :size="16" :stroke-width="1.8" aria-hidden="true" />
        <span>{{ $t(isAnalyzing ? 'viewer.analyzingColors' : 'viewer.analyzeColors') }}</span>
      </button>
      <button
        type="button"
        :class="{ 'is-active': pickerActive }"
        :disabled="!canPick"
        :aria-pressed="pickerActive"
        @click="emit('toggle-picker')"
      >
        <Pipette :size="16" :stroke-width="1.8" aria-hidden="true" />
        <span>{{ $t(pickerActive ? 'viewer.colorPickerActive' : 'viewer.colorPicker') }}</span>
      </button>
      </div>

    <section class="image-information__section" aria-labelledby="dominant-colors-title">
      <div class="image-information__section-heading">
        <h3 id="dominant-colors-title">{{ $t('viewer.dominantColors') }}</h3>
        <span v-if="metadata">{{ metadata.colorCount }}</span>
      </div>

      <p v-if="hasError" class="image-information__message image-information__message--error" role="status">
        {{ $t('viewer.colorAnalysisFailed') }}
      </p>
      <p v-else-if="!metadata && !isAnalyzing" class="image-information__message">
        {{ $t('viewer.colorAnalysisEmpty') }}
      </p>
      <div v-else-if="metadata" class="image-information__swatches">
        <button
          v-for="color in metadata.dominantColors"
          :key="`${color.hex}-${color.rgb}`"
          type="button"
          :class="{ 'is-selected': selectedColor?.hex === color.hex && selectedColor?.rgb === color.rgb }"
          :aria-pressed="selectedColor?.hex === color.hex && selectedColor?.rgb === color.rgb"
          :aria-label="`${color.hex}, ${color.percentage}%`"
          @click="emit('select-color', color)"
        >
          <span class="image-information__swatch" :style="{ backgroundColor: color.hex }" aria-hidden="true" />
          <span>{{ color.hex }}</span>
          <small>{{ color.percentage }}%</small>
        </button>
      </div>
    </section>

    <section class="image-information__section image-information__tags" aria-labelledby="image-tags-title">
      <div class="image-information__section-heading">
        <h3 id="image-tags-title">{{ $t('tags.imageTags') }}</h3>
        <span>{{ tags.length }}</span>
      </div>
      <div v-if="tags.length > 0" class="image-information__tag-list">
        <span v-for="tag in tags" :key="tag.id" class="image-information__tag" :title="tag.name">
          <span>{{ tag.name }}</span>
          <button type="button" :aria-label="$t('tags.removeTag', { name: tag.name })" @click="emit('remove-tag', tag.id)">
            <X :size="13" :stroke-width="2" aria-hidden="true" />
          </button>
        </span>
      </div>
      <p v-else class="image-information__message">{{ $t('tags.noImageTags') }}</p>
      <button type="button" class="image-information__manage-tags" @click="emit('manage-tags')">
        <Tags :size="16" :stroke-width="1.8" aria-hidden="true" />
        <span>{{ $t('viewer.manageTags') }}</span>
      </button>
    </section>

    <section class="image-information__section image-information__metadata" aria-labelledby="image-metadata-title">
      <div class="image-information__section-heading">
        <h3 id="image-metadata-title">{{ $t('viewer.basicImageInfo') }}</h3>
        <span v-if="imageMetadata?.status === 'ready'">{{ $t('viewer.metadataAvailable') }}</span>
      </div>
      <p
        v-if="metadataTaskStatus === 'idle' || metadataTaskStatus === 'running'"
        class="image-information__message"
        role="status"
        aria-busy="true"
      >
        {{ $t('viewer.loadingMetadata') }}
      </p>
      <template v-else-if="imageMetadata?.status === 'ready'">
        <dl>
          <dt>{{ $t('viewer.width') }}</dt>
          <dd>{{ imageMetadata.width ?? '—' }}</dd>
          <dt>{{ $t('viewer.height') }}</dt>
          <dd>{{ imageMetadata.height ?? '—' }}</dd>
          <dt>{{ $t('viewer.aspect') }}</dt>
          <dd>{{ formatAspectRatio(imageMetadata.width, imageMetadata.height) }}</dd>
          <dt>{{ $t('viewer.format') }}</dt>
          <dd>{{ imageMetadata.format ?? '—' }}</dd>
          <dt>{{ $t('viewer.alpha') }}</dt>
          <dd>{{ imageMetadata.hasAlpha ? $t('viewer.yes') : $t('viewer.no') }}</dd>
          <dt>{{ $t('viewer.metadata') }}</dt>
          <dd>{{ $t('viewer.metadataAvailable') }}</dd>
        </dl>
      </template>
      <div v-else class="image-information__metadata-error">
        <p class="image-information__message image-information__message--error" role="status">
          {{ $t(hasMetadataError ? 'viewer.metadataFailed' : 'viewer.metadataUnavailable') }}
        </p>
        <button
          type="button"
          :disabled="!canLoadMetadata"
          @click="emit('retry-metadata')"
        >
          <RefreshCw :size="16" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ $t('viewer.retryMetadata') }}</span>
        </button>
      </div>
    </section>

    <section class="image-information__section" aria-labelledby="duplicate-information-title">
      <div class="image-information__section-heading">
        <h3 id="duplicate-information-title">{{ $t('viewer.duplicateInformation') }}</h3>
        <span v-if="duplicateSummary">{{ duplicateSummary.totalCount }}</span>
      </div>
      <p v-if="hasDuplicatesError" class="image-information__message image-information__message--error" role="status">
        {{ $t('viewer.duplicateAnalysisFailed') }}
      </p>
      <p v-else-if="!duplicateSummary && !isDuplicatesLoading" class="image-information__message">
        {{ $t('viewer.duplicateAnalysisEmpty') }}
      </p>
      <p v-else-if="isDuplicatesLoading && !duplicateSummary" class="image-information__message" aria-busy="true">
        {{ $t('viewer.analyzingDuplicates') }}
      </p>
      <button
        v-else-if="duplicateSummary"
        class="image-information__duplicate-summary"
        type="button"
        :disabled="!canViewDuplicates"
        @click="emit('open-duplicates')"
      >
        <span>{{ $t('viewer.exactDuplicates') }} <strong>{{ duplicateSummary.exactCount }}</strong></span>
        <span>{{ $t('viewer.similarCandidates') }} <strong>{{ duplicateSummary.similarCount }}</strong></span>
        <small>{{ $t('viewer.viewDuplicateCandidates') }}</small>
      </button>
    </section>

    <section v-if="detailColor" class="image-information__section image-information__selection" aria-live="polite">
      <div class="image-information__section-heading">
        <h3>{{ pickedColor ? $t('viewer.pixelColor') : $t('viewer.selectedColor') }}</h3>
        <span class="image-information__selected-swatch" :style="{ backgroundColor: detailColor.hex }" aria-hidden="true" />
      </div>
      <dl>
        <template v-if="pickedColor">
          <dt>{{ $t('viewer.coordinates') }}</dt>
          <dd>{{ pickedColor.x }}, {{ pickedColor.y }}</dd>
        </template>
        <dt>HEX</dt>
        <dd>{{ detailColor.hex }}</dd>
        <dt>RGB</dt>
        <dd>{{ detailColor.rgb }}</dd>
        <dt>HSL</dt>
        <dd>{{ detailColor.hsl }}</dd>
        <template v-if="!pickedColor">
          <dt>{{ $t('viewer.percentage') }}</dt>
          <dd>{{ detailColor.percentage }}%</dd>
        </template>
      </dl>
    </section>

    <section v-if="detailColor" class="image-information__section" aria-label="$t('viewer.colorWheel')">
      <div class="image-information__section-heading">
        <h3>{{ $t('viewer.colorWheel') }}</h3>
        <span>{{ $t('viewer.colorWheelDescription') }}</span>
      </div>
      <div class="image-information__wheel" aria-hidden="true">
        <div class="image-information__hue-ring">
          <span class="image-information__hue-marker" :style="hueMarkerStyle" />
        </div>
        <div class="image-information__sv-box" :style="saturationValueStyle">
          <span class="image-information__sv-marker" :style="saturationMarkerStyle" />
        </div>
      </div>
    </section>
    </div>
  </aside>
</template>

<style scoped lang="scss">
.image-information {
  position: absolute;
  inset: 0 0 0 auto;
  z-index: var(--iv-layer-shell);
  display: flex;
  flex-direction: column;
  width: min(var(--iv-details-panel-width), 100%);
  min-width: min(300px, 100%);
  height: 100%;
  min-height: 0;
  overflow: hidden;
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-glass-bg-strong);
  border-left: 1px solid var(--iv-overlay-border);
  box-shadow: var(--iv-shadow-floating);
}

.image-information__scroll-content {
  flex: 1 1 auto;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
}

.image-information__header,
.image-information__section,
.image-information__actions {
  padding-right: var(--iv-space-4);
  padding-left: var(--iv-space-4);
}

.image-information__header {
  flex: 0 0 auto;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--iv-space-3);
  padding-top: var(--iv-space-5);
  padding-bottom: var(--iv-space-4);
}

.image-information__eyebrow,
.image-information__header h2,
.image-information__section h3,
.image-information__message,
.image-information__section-heading span,
.image-information__selection dl {
  margin: 0;
}

.image-information__metadata dl {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: var(--iv-space-1) var(--iv-space-3);
  margin: 0;
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.image-information__metadata dt {
  color: var(--iv-overlay-text-secondary);
}

.image-information__metadata dd {
  margin: 0;
  font-family: ui-monospace, "Cascadia Code", monospace;
  font-variant-numeric: tabular-nums;
}

.image-information__metadata-error {
  display: grid;
  gap: var(--iv-space-3);
}

.image-information__metadata-error button {
  width: fit-content;
  padding: 0 var(--iv-space-3);
}

.image-information__eyebrow,
.image-information__section-heading span {
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.image-information__header h2 {
  max-width: 224px;
  overflow: hidden;
  font-size: var(--iv-font-size-16);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-22);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.image-information button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--iv-space-2);
  min-width: 0;
  min-height: var(--iv-control-height);
  color: var(--iv-overlay-text-primary);
  font: inherit;
  font-size: var(--iv-font-size-13);
  background-color: var(--iv-overlay-surface);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: background-color var(--iv-motion-fast) ease, border-color var(--iv-motion-fast) ease;
}

.image-information__header > button {
  flex: 0 0 var(--iv-control-height);
  width: var(--iv-control-height);
  padding: 0;
}

.image-information button:hover:not(:disabled) {
  background-color: var(--iv-overlay-surface-hover);
}

.image-information button:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.image-information button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.image-information__actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--iv-space-2);
  padding-bottom: var(--iv-space-4);
  border-bottom: 1px solid var(--iv-overlay-border);
}

.image-information__actions button {
  padding: 0 var(--iv-space-2);
}

.image-information__actions button.is-active {
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-overlay-accent-active);
  border-color: var(--iv-accent);
}

.image-information__spinner {
  animation: image-information-spin 0.8s linear infinite;
}

.image-information__section {
  display: grid;
  gap: var(--iv-space-3);
  padding-top: var(--iv-space-4);
  padding-bottom: var(--iv-space-4);
  border-bottom: 1px solid var(--iv-overlay-border);
}

.image-information__section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-2);
}

.image-information__section-heading > span {
  min-width: 0;
  max-width: 52%;
  overflow: hidden;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.image-information__section h3 {
  font-size: var(--iv-font-size-13);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-18);
}

.image-information__message {
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.image-information__message--error {
  color: var(--iv-danger);
}

.image-information__tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: var(--iv-space-2);
  min-width: 0;
}

.image-information__tag {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  max-width: 100%;
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-overlay-accent-active);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-full);
}

.image-information__tag > span {
  max-width: 156px;
  padding: 2px var(--iv-space-2);
  overflow: hidden;
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.image-information__tag > button {
  min-width: var(--iv-control-small-height) !important;
  width: var(--iv-control-small-height);
  min-height: var(--iv-control-small-height) !important;
  padding: 0;
  background: transparent;
  border: 0;
  border-left: 1px solid var(--iv-overlay-border);
  border-radius: 0 var(--iv-radius-full) var(--iv-radius-full) 0;
}

.image-information__manage-tags {
  justify-content: flex-start !important;
  width: fit-content !important;
  padding: 0 var(--iv-space-3);
}

.image-information__swatches {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--iv-space-2);
}

.image-information__duplicate-summary {
  display: grid !important;
  grid-template-columns: minmax(0, 1fr) auto;
  justify-items: start !important;
  width: 100%;
  min-height: auto !important;
  padding: var(--iv-space-3) !important;
  text-align: left;
}

.image-information__duplicate-summary span {
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.image-information__duplicate-summary strong {
  color: var(--iv-overlay-text-primary);
  font-variant-numeric: tabular-nums;
}

.image-information__duplicate-summary small {
  grid-column: 1 / -1;
  margin-top: var(--iv-space-2);
  color: var(--iv-accent-hover);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.image-information__swatches button {
  display: grid;
  grid-template-columns: var(--iv-space-4) minmax(0, 1fr) auto;
  gap: var(--iv-space-2);
  min-height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  text-align: left;
}

.image-information__swatches button.is-selected {
  background-color: var(--iv-overlay-accent-active);
  border-color: var(--iv-accent);
}

.image-information__swatches button > span:not(.image-information__swatch) {
  overflow: hidden;
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.image-information__swatches small {
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-12);
  font-variant-numeric: tabular-nums;
}

.image-information__swatch,
.image-information__selected-swatch {
  display: block;
  width: var(--iv-space-4);
  height: var(--iv-space-4);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-xs);
}

.image-information__selection dl {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: var(--iv-space-1) var(--iv-space-3);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.image-information__selection dt {
  color: var(--iv-overlay-text-secondary);
}

.image-information__selection dd {
  margin: 0;
  overflow-wrap: anywhere;
  font-family: ui-monospace, "Cascadia Code", monospace;
  font-variant-numeric: tabular-nums;
}

.image-information__wheel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: var(--iv-space-3);
  align-items: center;
}

.image-information__hue-ring,
.image-information__sv-box {
  position: relative;
  aspect-ratio: 1;
}

.image-information__hue-ring {
  background: conic-gradient(
    #F00,
    #FF0,
    #0F0,
    #0FF,
    #00F,
    #F0F,
    #F00
  );
  border-radius: var(--iv-radius-round);
}

.image-information__hue-ring::before {
  position: absolute;
  inset: 16%;
  content: "";
  background: var(--iv-overlay-surface-strong);
  border: 1px solid var(--iv-overlay-border);
  border-radius: inherit;
}

.image-information__hue-marker,
.image-information__sv-marker {
  position: absolute;
  width: var(--iv-space-3);
  height: var(--iv-space-3);
  pointer-events: none;
  background: transparent;
  border: 2px solid #FFF;
  border-radius: var(--iv-radius-round);
  box-shadow: 0 0 0 1px rgb(0 0 0 / 70%);
  transform: translate(-50%, -50%);
}

.image-information__sv-box {
  background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #FFF, hsl(var(--iv-color-wheel-hue) 100% 50%));
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
}

@keyframes image-information-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .image-information button {
    transition: none;
  }

  .image-information__spinner {
    animation: none;
  }
}
</style>
