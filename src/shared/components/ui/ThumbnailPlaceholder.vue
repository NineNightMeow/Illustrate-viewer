<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    variant?: 0 | 1 | 2;
  }>(),
  {
    variant: 0,
  },
);
</script>

<template>
  <div
    class="thumbnail-placeholder"
    :class="`thumbnail-placeholder--${variant}`"
    role="img"
    :aria-label="label"
  >
    <span class="thumbnail-placeholder__frame" aria-hidden="true" />
    <span class="thumbnail-placeholder__detail" aria-hidden="true" />
  </div>
</template>

<style scoped lang="scss">
.thumbnail-placeholder {
  position: relative;
  display: block;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background-color: var(--iv-placeholder-surface);
}

.thumbnail-placeholder::before,
.thumbnail-placeholder::after,
.thumbnail-placeholder__frame,
.thumbnail-placeholder__detail {
  position: absolute;
  display: block;
  content: "";
  border: 1px solid var(--iv-placeholder-line);
}

.thumbnail-placeholder::before {
  inset: 18% 12%;
  border-radius: var(--iv-radius-sm);
  transform: rotate(-6deg);
}

.thumbnail-placeholder::after {
  right: 8%;
  bottom: 10%;
  left: 24%;
  height: 1px;
  border: 0;
  background-color: var(--iv-placeholder-line-strong);
  box-shadow: 0 calc(-1 * var(--iv-space-2)) 0 var(--iv-placeholder-line);
}

.thumbnail-placeholder__frame {
  top: 24%;
  left: 22%;
  width: 28%;
  height: 34%;
  border-color: var(--iv-placeholder-line-strong);
  border-radius: var(--iv-radius-round);
}

.thumbnail-placeholder__detail {
  right: 20%;
  bottom: 24%;
  width: 30%;
  height: 16%;
  border-width: 1px 0 0;
  transform: skewX(-24deg);
}

.thumbnail-placeholder--1::before {
  inset: 12% 20%;
  border-radius: 45% 45% var(--iv-radius-md) var(--iv-radius-md);
  transform: rotate(8deg);
}

.thumbnail-placeholder--1 .thumbnail-placeholder__frame {
  top: 20%;
  left: 38%;
  width: 24%;
  height: 24%;
}

.thumbnail-placeholder--2::before {
  inset: 22% 14% 16%;
  border-radius: var(--iv-radius-round) var(--iv-radius-sm) var(--iv-radius-round) var(--iv-radius-sm);
  transform: rotate(4deg);
}

.thumbnail-placeholder--2 .thumbnail-placeholder__frame {
  top: 30%;
  left: 18%;
  width: 42%;
  height: 20%;
  border-radius: var(--iv-radius-xs);
}
</style>
