import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export function addFavorite(imageId: string) {
  return invoke<void>("add_favorite", { imageId });
}

export function removeFavorite(imageId: string) {
  return invoke<void>("remove_favorite", { imageId });
}

export function isFavorite(imageId: string) {
  return invoke<boolean>("is_favorite", { imageId });
}

export function listFavorites() {
  return invoke<ImageAsset[]>("list_favorites");
}
