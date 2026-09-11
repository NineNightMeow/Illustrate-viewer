import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import { getImageAssetErrorCode, prepareImageAsset, type ImageAssetErrorCode } from "../library/imageAssetApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let requestSequence = 0;

export const useQuickPreviewStore = defineStore("quick-preview", {
  state: () => ({
    isOpen: false,
    currentImageId: null as string | null,
    sourcePath: null as string | null,
    isLoading: false,
    hasError: false,
    errorCode: null as ImageAssetErrorCode | null,
    scale: 1,
    translateX: 0,
    translateY: 0,
  }),
  actions: {
    async open(asset: ImageAsset) {
      const request = ++requestSequence;
      this.isOpen = true;
      this.currentImageId = asset.id;
      this.sourcePath = null;
      this.isLoading = true;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();

      if (!isTauriEnvironment) {
        this.sourcePath = asset.path;
        this.isLoading = false;
        return;
      }

      try {
        const sourcePath = await prepareImageAsset(asset);
        if (request !== requestSequence || !this.isOpen || this.currentImageId !== asset.id) return;
        this.sourcePath = sourcePath;
      } catch (error) {
        if (request !== requestSequence || !this.isOpen || this.currentImageId !== asset.id) return;
        this.hasError = true;
        this.errorCode = getImageAssetErrorCode(error);
      } finally {
        if (request === requestSequence && this.currentImageId === asset.id) {
          this.isLoading = false;
        }
      }
    },
    close() {
      requestSequence += 1;
      this.isOpen = false;
      this.currentImageId = null;
      this.sourcePath = null;
      this.isLoading = false;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();
    },
    resetTransform(scale = 1) {
      this.scale = scale;
      this.translateX = 0;
      this.translateY = 0;
    },
    setTransform(transform: { scale: number; translateX: number; translateY: number }) {
      this.scale = transform.scale;
      this.translateX = transform.translateX;
      this.translateY = transform.translateY;
    },
  },
});
