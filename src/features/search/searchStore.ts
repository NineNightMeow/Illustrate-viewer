import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import { filterImages, type ImageOrientation, type SearchFilters } from "./searchApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let searchRequestSequence = 0;

export type SearchViewContext = {
  query: string;
  filters: SearchFilters;
};

const emptyFilters = (): SearchFilters => ({
  libraryId: null,
  collectionId: null,
  tagIds: [],
  favoritesOnly: false,
  formats: [],
  metadata: {},
  smartCollectionId: null,
});

function cloneFilters(filters: SearchFilters): SearchFilters {
  return {
    ...filters,
    tagIds: [...filters.tagIds],
    formats: [...filters.formats],
    metadata: { ...filters.metadata },
  };
}

export const useSearchStore = defineStore("search", {
  state: () => ({
    query: "",
    filters: emptyFilters(),
    assets: [] as ImageAsset[],
    isLoading: false,
    hasSearched: false,
    error: null as "search_persistence_failed" | null,
  }),
  getters: {
    hasCriteria: (state) => Boolean(
      state.query.trim()
      || state.filters.libraryId
      || state.filters.collectionId
      || state.filters.tagIds.length > 0
      || state.filters.favoritesOnly
      || state.filters.formats.length > 0
      || state.filters.metadata.minimumWidth
      || state.filters.metadata.orientation
      || state.filters.smartCollectionId,
    ),
  },
  actions: {
    snapshotViewContext(): SearchViewContext {
      return { query: this.query, filters: cloneFilters(this.filters) };
    },
    restoreViewContext(context: SearchViewContext) {
      this.query = context.query;
      this.filters = cloneFilters(context.filters);
    },
    async search() {
      const request = ++searchRequestSequence;
      if (!this.hasCriteria) {
        this.assets = [];
        this.hasSearched = false;
        return [] as ImageAsset[];
      }
      if (!isTauriEnvironment) return [] as ImageAsset[];

      this.isLoading = true;
      this.error = null;
      try {
        const assets = await filterImages(this.query, cloneFilters(this.filters));
        if (request !== searchRequestSequence) return this.assets;

        this.assets = assets;
        this.hasSearched = true;
        return this.assets;
      } catch {
        if (request !== searchRequestSequence) return this.assets;

        this.assets = [];
        this.error = "search_persistence_failed";
        this.hasSearched = true;
        return [] as ImageAsset[];
      } finally {
        if (request === searchRequestSequence) this.isLoading = false;
      }
    },
    clear() {
      searchRequestSequence += 1;
      this.query = "";
      this.filters = emptyFilters();
      this.assets = [];
      this.isLoading = false;
      this.error = null;
      this.hasSearched = false;
    },
    setFormat(format: string | null) {
      this.filters.formats = format ? [format] : [];
    },
    setMinimumWidth(width: number | null) {
      this.filters.metadata = { ...this.filters.metadata, minimumWidth: width };
    },
    setOrientation(orientation: ImageOrientation | null) {
      this.filters.metadata = { ...this.filters.metadata, orientation };
    },
  },
});
