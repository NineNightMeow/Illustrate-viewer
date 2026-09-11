import { defineStore } from "pinia";
import type { ImageAsset } from "./types";

const MAX_RECENT_IMAGES = 8;

type RecentImageVisit = {
  imageId: string;
  libraryId: string;
  visitedAt: number;
};

export const useRecentImageStore = defineStore("recent-image", {
  state: () => ({
    entries: [] as RecentImageVisit[],
  }),
  actions: {
    recordViewed(asset: ImageAsset) {
      this.entries = [
        { imageId: asset.id, libraryId: asset.libraryId, visitedAt: Date.now() },
        ...this.entries.filter((entry) => entry.imageId !== asset.id),
      ].slice(0, MAX_RECENT_IMAGES);
    },
  },
});
