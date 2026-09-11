<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, ref, watch } from "vue";
import { prepareCustomBackground } from "../features/settings/customBackgroundApi";

const props = defineProps<{
  enabled: boolean;
  source: string | null;
  blur: number;
}>();

const emit = defineEmits<{
  "source-status": [status: "idle" | "available" | "unavailable"];
}>();

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const resolvedSource = ref<string | null>(null);
const imageFailed = ref(false);
let sourceRequest = 0;

const shouldRender = computed(() => props.enabled && Boolean(resolvedSource.value) && !imageFailed.value);

watch(
  () => props.source,
  async (source) => {
    const request = ++sourceRequest;
    resolvedSource.value = null;
    imageFailed.value = false;

    if (!source) {
      emit("source-status", "idle");
      return;
    }

    if (!isTauriEnvironment) {
      emit("source-status", "idle");
      return;
    }

    try {
      const path = await prepareCustomBackground(source);
      if (request !== sourceRequest) return;
      resolvedSource.value = convertFileSrc(path);
      emit("source-status", "available");
    } catch {
      if (request === sourceRequest) emit("source-status", "unavailable");
    }
  },
  { immediate: true },
);

function handleImageError() {
  imageFailed.value = true;
  emit("source-status", "unavailable");
}
</script>

<template>
  <div
    v-if="shouldRender"
    class="custom-background-layer"
    :class="{ 'is-blurred': blur > 0 }"
    aria-hidden="true"
  >
    <img :src="resolvedSource ?? undefined" alt="" @error="handleImageError">
    <span class="custom-background-layer__dim" />
  </div>
</template>

<style scoped lang="scss">
.custom-background-layer {
  position: absolute;
  z-index: 0;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}

.custom-background-layer img {
  width: 100%;
  height: 100%;
  object-fit: var(--iv-background-image-fit);
  opacity: var(--iv-background-image-opacity);
  filter: blur(var(--iv-background-image-blur));
}

.custom-background-layer.is-blurred img {
  transform: scale(1.04);
}

.custom-background-layer__dim {
  position: absolute;
  inset: 0;
  background-color: var(--iv-background-image-dim-color);
  opacity: var(--iv-background-image-dim);
}
</style>
