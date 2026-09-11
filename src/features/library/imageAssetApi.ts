import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "./types";

export type ImageAssetErrorCode =
  | "source_missing"
  | "source_unreadable"
  | "decode_failed"
  | "unsupported_format"
  | "library_unavailable"
  | "library_not_found"
  | "unknown";

export function getImageAssetErrorCode(error: unknown): ImageAssetErrorCode {
  const value = parseImageAssetError(error);
  const code = value?.code;
  switch (code) {
    case "source_missing":
    case "source_unreadable":
    case "decode_failed":
    case "unsupported_format":
    case "library_unavailable":
    case "library_not_found":
      return code;
    default:
      return "unknown";
  }
}

function parseImageAssetError(error: unknown): { code?: unknown } | null {
  if (typeof error === "object" && error !== null) return error as { code?: unknown };
  if (typeof error !== "string") return null;

  try {
    const parsed = JSON.parse(error);
    return typeof parsed === "object" && parsed !== null ? parsed as { code?: unknown } : null;
  } catch {
    return { code: error };
  }
}

export function prepareImageAsset(asset: ImageAsset) {
  return invoke<string>("prepare_image_asset", {
    asset: {
      imageId: asset.id,
      libraryId: asset.libraryId,
      sourcePath: asset.path,
    },
  });
}
