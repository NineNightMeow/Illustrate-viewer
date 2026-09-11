import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export type Collection = {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
};

export type CollectionMembership = {
  collectionId: string;
  assignedCount: number;
};

export function createCollection(name: string) {
  return invoke<Collection>("create_collection", { name });
}

export function deleteCollection(collectionId: string) {
  return invoke<void>("delete_collection", { collectionId });
}

export function renameCollection(collectionId: string, name: string) {
  return invoke<Collection>("rename_collection", { collectionId, name });
}

export function listCollections() {
  return invoke<Collection[]>("list_collections");
}

export function addImageToCollection(collectionId: string, imageId: string) {
  return invoke<void>("add_image_to_collection", { collectionId, imageId });
}

export function listCollectionMemberships(collectionIds: string[], imageIds: string[]) {
  return invoke<CollectionMembership[]>("list_collection_memberships", { collectionIds, imageIds });
}

export function addImagesToCollections(collectionIds: string[], imageIds: string[]) {
  return invoke<void>("add_images_to_collections", { collectionIds, imageIds });
}

export function createCollectionWithImages(name: string, imageIds: string[]) {
  return invoke<Collection>("create_collection_with_images", { name, imageIds });
}

export function removeImageFromCollection(collectionId: string, imageId: string) {
  return invoke<void>("remove_image_from_collection", { collectionId, imageId });
}

export function listCollectionImages(collectionId: string) {
  return invoke<ImageAsset[]>("list_collection_images", { collectionId });
}
