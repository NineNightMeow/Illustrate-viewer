import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export type SearchFilters = {
  libraryId: string | null;
  collectionId: string | null;
  tagIds: string[];
  favoritesOnly: boolean;
  formats: string[];
  metadata: ImageMetadataSearchConditions;
  smartCollectionId: string | null;
};

export type ImageOrientation = "landscape" | "portrait" | "square";

export type ImageMetadataSearchConditions = {
  minimumWidth?: number | null;
  maximumWidth?: number | null;
  minimumHeight?: number | null;
  maximumHeight?: number | null;
  minimumAspectRatio?: number | null;
  maximumAspectRatio?: number | null;
  minimumFileSize?: number | null;
  maximumFileSize?: number | null;
  hasAlpha?: boolean | null;
  orientation?: ImageOrientation | null;
};

export type ImageSearchQuery = {
  filename?: string | null;
  path?: string | null;
  formats?: string[];
  metadata?: ImageMetadataSearchConditions;
  favorite?: boolean | null;
};

export function filterImages(query: string, filters: SearchFilters) {
  return invoke<ImageAsset[]>("filter_images", {
    filters: {
      query: query.trim() || null,
      ...filters,
    },
  });
}

export function queryImageAssets(query: ImageSearchQuery) {
  return invoke<ImageAsset[]>("query_image_assets", { query });
}
