import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export type SmartCollectionRule =
  | { kind: "largeImages"; minimumPixels: number }
  | { kind: "minimumWidth"; minimumWidth: number }
  | { kind: "minimumHeight"; minimumHeight: number }
  | { kind: "format"; extension: string }
  | { kind: "landscape" }
  | { kind: "portrait" }
  | { kind: "transparentPng" }
  | { kind: "favorite" };

export type SmartCollection = {
  id: string;
  name: string;
  rule: SmartCollectionRule;
  createdAt: number;
  updatedAt: number;
};

export function createSmartCollection(name: string, rule: SmartCollectionRule) {
  return invoke<SmartCollection>("create_smart_collection", { name, rule });
}

export function deleteSmartCollection(collectionId: string) {
  return invoke<void>("delete_smart_collection", { collectionId });
}

export function renameSmartCollection(collectionId: string, name: string) {
  return invoke<SmartCollection>("rename_smart_collection", { collectionId, name });
}

export function listSmartCollections() {
  return invoke<SmartCollection[]>("list_smart_collections");
}

export function listSmartCollectionImages(collectionId: string) {
  return invoke<ImageAsset[]>("list_smart_collection_images", { collectionId });
}
