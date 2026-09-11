import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export type DuplicateMatchKind = "exact" | "similar";

export type DuplicateIndicator = {
  imageId: string;
  matchKind: DuplicateMatchKind;
};

export type DuplicateCandidate = {
  asset: ImageAsset;
  matchKind: DuplicateMatchKind;
  perceptualDistance: number;
};

export type DuplicateCandidatePage = {
  exactCount: number;
  similarCount: number;
  totalCount: number;
  offset: number;
  candidates: DuplicateCandidate[];
};

export function getDuplicateIndicators(imageIds: string[]) {
  return invoke<DuplicateIndicator[]>("get_duplicate_indicators", { imageIds });
}

export function listDuplicateCandidates(imageId: string, offset: number, limit: number) {
  return invoke<DuplicateCandidatePage>("list_duplicate_candidates", { imageId, offset, limit });
}
