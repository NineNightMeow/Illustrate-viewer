<template>
  <div id="overlay-root" class="overlay-root" aria-live="polite" />
</template>

<style scoped lang="scss">
.overlay-root {
  position: fixed;
  inset: 0;
  z-index: var(--iv-layer-critical);
  pointer-events: none;
}
</style>
