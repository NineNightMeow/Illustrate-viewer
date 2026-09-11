import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import type { SearchViewContext } from "../search/searchStore";
import { getImageAssetErrorCode, prepareImageAsset, type ImageAssetErrorCode } from "../library/imageAssetApi";

export type ViewerBackground = "auto" | "dark" | "light";
export type ViewerRotation = 0 | 90 | 180 | 270;
export type ViewerSource = "library" | "search" | "collection" | "favorites" | "smart-collection" | "tag" | "home";

export type ViewerResultContext = {
  source: ViewerSource;
  entryLibraryId: string;
  resultImageIds: string[];
  imageLibraryIds: Record<string, string>;
  activeImageId: string;
  returnPath: string;
  sourceId: string | null;
  selectedImageIds: string[];
  scrollTop: number;
  sort?: "name" | "createdAt" | "modifiedAt";
  fileType?: string;
  layoutMode?: "grid";
  thumbnailSize?: number | null;
  search?: SearchViewContext;
};

export type ViewerReturnContext = Omit<
  ViewerResultContext,
  "entryLibraryId" | "resultImageIds" | "imageLibraryIds"
>;

export function imageLibraryIdsFor(assets: Pick<ImageAsset, "id" | "libraryId">[]) {
  return Object.fromEntries(assets.map((asset) => [asset.id, asset.libraryId]));
}

function cloneSearchContext(context: SearchViewContext | undefined) {
  if (!context) return undefined;

  return {
    query: context.query,
    filters: {
      ...context.filters,
      tagIds: [...context.filters.tagIds],
      formats: [...context.filters.formats],
      metadata: { ...context.filters.metadata },
    },
  };
}

function cloneReturnContext(context: ViewerResultContext): ViewerReturnContext {
  return {
    source: context.source,
    activeImageId: context.activeImageId,
    returnPath: context.returnPath,
    sourceId: context.sourceId,
    selectedImageIds: [...context.selectedImageIds],
    scrollTop: context.scrollTop,
    sort: context.sort,
    fileType: context.fileType,
    layoutMode: context.layoutMode,
    thumbnailSize: context.thumbnailSize,
    search: cloneSearchContext(context.search),
  };
}

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let requestSequence = 0;

export const useViewerStore = defineStore("viewer", {
  state: () => ({
    context: null as ViewerResultContext | null,
    sourcePath: null as string | null,
    sourceImageId: null as string | null,
    isLoading: false,
    hasError: false,
    errorCode: null as ImageAssetErrorCode | null,
    background: "auto" as ViewerBackground,
    zoom: 1,
    panX: 0,
    panY: 0,
    rotation: 0 as ViewerRotation,
  }),
  getters: {
    entryLibraryId: (state) => state.context?.entryLibraryId ?? null,
    currentImageId: (state) => state.context?.activeImageId ?? null,
    resultImageIds: (state) => state.context?.resultImageIds ?? [],
    currentIndex: (state) => state.context
      ? state.context.resultImageIds.indexOf(state.context.activeImageId)
      : -1,
    libraryIdFor: (state) => (imageId: string | null | undefined) =>
      imageId ? state.context?.imageLibraryIds[imageId] ?? null : null,
  },
  actions: {
    open(input: ViewerResultContext) {
      const resultImageIds = [...input.resultImageIds];
      if (!resultImageIds.includes(input.activeImageId)) return;

      requestSequence += 1;
      this.context = {
        ...input,
        resultImageIds,
        imageLibraryIds: { ...input.imageLibraryIds },
        selectedImageIds: [...input.selectedImageIds],
        search: cloneSearchContext(input.search),
      };
      this.sourcePath = null;
      this.sourceImageId = null;
      this.isLoading = false;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();
    },
    selectImage(imageId: string) {
      if (!this.context?.resultImageIds.includes(imageId)) return;

      requestSequence += 1;
      this.context.activeImageId = imageId;
      this.sourcePath = null;
      this.sourceImageId = null;
      this.isLoading = false;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();
    },
    async prepareCurrent(asset: ImageAsset) {
      const request = ++requestSequence;
      this.sourcePath = null;
      this.sourceImageId = null;
      this.isLoading = true;
      this.hasError = false;
      this.errorCode = null;

      if (!isTauriEnvironment) {
        this.sourcePath = asset.path;
        this.sourceImageId = asset.id;
        this.isLoading = false;
        return;
      }

      try {
        const sourcePath = await prepareImageAsset(asset);
        if (request !== requestSequence || this.currentImageId !== asset.id) return;
        this.sourcePath = sourcePath;
        this.sourceImageId = asset.id;
      } catch (error) {
        if (request !== requestSequence || this.currentImageId !== asset.id) return;
        this.hasError = true;
        this.errorCode = getImageAssetErrorCode(error);
      } finally {
        if (request === requestSequence && this.currentImageId === asset.id) {
          this.isLoading = false;
        }
      }
    },
    async preparePrefetch(asset: ImageAsset) {
      if (!isTauriEnvironment) return asset.path;

      try {
        return await prepareImageAsset(asset);
      } catch {
        return null;
      }
    },
    setBackground(background: ViewerBackground) {
      this.background = background;
    },
    complete() {
      if (!this.context) return null;

      const returnContext = cloneReturnContext(this.context);
      requestSequence += 1;
      this.sourcePath = null;
      this.sourceImageId = null;
      this.isLoading = false;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();
      return returnContext;
    },
    consumeReturnContext(source: ViewerSource, sourceId: string | null = null) {
      if (!this.context || this.context.source !== source || this.context.sourceId !== sourceId) return null;

      const context = cloneReturnContext(this.context);
      this.context = null;
      return context;
    },
    setZoom(zoom: number) {
      this.zoom = zoom;
    },
    setPan(panX: number, panY: number) {
      this.panX = panX;
      this.panY = panY;
    },
    rotateClockwise() {
      this.rotation = ((this.rotation + 90) % 360) as ViewerRotation;
    },
    resetTransform() {
      this.zoom = 1;
      this.panX = 0;
      this.panY = 0;
      this.rotation = 0;
    },
    close() {
      requestSequence += 1;
      this.context = null;
      this.sourcePath = null;
      this.sourceImageId = null;
      this.isLoading = false;
      this.hasError = false;
      this.errorCode = null;
      this.resetTransform();
    },
  },
});
