import { defineStore } from "pinia";

const MINIMUM_OPACITY = 20;
const MAXIMUM_OPACITY = 100;

export type ReferenceWindowState = {
  mirrored: boolean;
  opacity: number;
  alwaysOnTop: boolean;
  locked: boolean;
};

export const useReferenceWindowStore = defineStore("reference-window", {
  state: () => ({
    mirrored: false,
    opacity: MAXIMUM_OPACITY,
    alwaysOnTop: false,
    locked: false,
  }),
  actions: {
    hydrate(state: ReferenceWindowState) {
      this.mirrored = state.mirrored;
      this.opacity = Math.max(MINIMUM_OPACITY, Math.min(MAXIMUM_OPACITY, state.opacity));
      this.alwaysOnTop = state.alwaysOnTop;
      this.locked = state.locked;
    },
    toggleMirror() {
      this.mirrored = !this.mirrored;
    },
    setOpacity(opacity: number) {
      this.opacity = Math.max(MINIMUM_OPACITY, Math.min(MAXIMUM_OPACITY, opacity));
    },
    setAlwaysOnTop(alwaysOnTop: boolean) {
      this.alwaysOnTop = alwaysOnTop;
    },
    setLocked(locked: boolean) {
      this.locked = locked;
    },
  },
});
