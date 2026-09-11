import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import { addFavorite, listFavorites, removeFavorite } from "./favoriteApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type FavoriteErrorCode = "favorite_image_not_found" | "favorite_persistence_failed";

function parseError(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function errorCode(error: unknown): FavoriteErrorCode {
  const value = typeof error === "string" ? parseError(error) : error;
  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (code === "favorite_image_not_found" || code === "favorite_persistence_failed") return code;
  }
  return "favorite_persistence_failed";
}

export const useFavoriteStore = defineStore("favorite", {
  state: () => ({
    favoriteIds: [] as string[],
    pendingIds: [] as string[],
    isLoading: false,
    error: null as FavoriteErrorCode | null,
  }),
  actions: {
    isFavorite(imageId: string) {
      return this.favoriteIds.includes(imageId);
    },
    isPending(imageId: string) {
      return this.pendingIds.includes(imageId);
    },
    async load() {
      if (!isTauriEnvironment) return [] as ImageAsset[];

      this.isLoading = true;
      this.error = null;
      try {
        const assets = await listFavorites();
        this.favoriteIds = assets.map((asset) => asset.id);
        return assets;
      } catch (error) {
        this.error = errorCode(error);
        return [] as ImageAsset[];
      } finally {
        this.isLoading = false;
      }
    },
    async toggle(imageId: string) {
      if (!isTauriEnvironment || this.isPending(imageId)) return this.isFavorite(imageId);

      const wasFavorite = this.isFavorite(imageId);
      const previousIds = this.favoriteIds;
      this.favoriteIds = wasFavorite
        ? this.favoriteIds.filter((id) => id !== imageId)
        : [...this.favoriteIds, imageId];
      this.pendingIds = [...this.pendingIds, imageId];
      this.error = null;

      try {
        if (wasFavorite) await removeFavorite(imageId);
        else await addFavorite(imageId);
      } catch (error) {
        this.favoriteIds = previousIds;
        this.error = errorCode(error);
      } finally {
        this.pendingIds = this.pendingIds.filter((id) => id !== imageId);
      }

      return this.isFavorite(imageId);
    },
  },
});
