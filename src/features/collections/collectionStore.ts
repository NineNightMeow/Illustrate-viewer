import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import {
  addImageToCollection,
  addImagesToCollections,
  createCollection,
  createCollectionWithImages,
  deleteCollection,
  listCollectionImages,
  listCollections,
  listCollectionMemberships,
  removeImageFromCollection,
  renameCollection,
  type Collection,
  type CollectionMembership,
} from "./collectionApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type CollectionSummary = Collection & { imageCount: number };
export type CollectionErrorCode =
  | "invalid_collection_name"
  | "duplicate_collection_name"
  | "collection_not_found"
  | "collection_image_not_found"
  | "collection_persistence_failed"
  | "empty_collection_selection"
  | "empty_image_selection";

function parseError(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function errorCode(error: unknown): CollectionErrorCode {
  const value = typeof error === "string" ? parseError(error) : error;
  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "invalid_collection_name"
      || code === "duplicate_collection_name"
      || code === "collection_not_found"
      || code === "collection_image_not_found"
      || code === "collection_persistence_failed"
      || code === "empty_collection_selection"
      || code === "empty_image_selection"
    ) return code;
  }
  return "collection_persistence_failed";
}

function sortCollections(collections: CollectionSummary[]) {
  return [...collections].sort((left, right) => right.updatedAt - left.updatedAt);
}

function normalizedName(name: string) {
  return name.trim().toLocaleLowerCase();
}

export const useCollectionStore = defineStore("collection", {
  state: () => ({
    collections: [] as CollectionSummary[],
    error: null as CollectionErrorCode | null,
    isLoading: false,
    workingId: null as string | null,
  }),
  actions: {
    async load() {
      if (!isTauriEnvironment) return [] as CollectionSummary[];

      this.isLoading = true;
      this.error = null;
      try {
        const collections = await listCollections();
        const summaries = await Promise.all(collections.map(async (collection) => ({
          ...collection,
          imageCount: (await listCollectionImages(collection.id)).length,
        })));
        this.collections = sortCollections(summaries);
        return this.collections;
      } catch (error) {
        this.error = errorCode(error);
        return [] as CollectionSummary[];
      } finally {
        this.isLoading = false;
      }
    },
    async create(name: string) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_collection_name";
        return null;
      }
      if (this.collections.some((collection) => normalizedName(collection.name) === nameKey)) {
        this.error = "duplicate_collection_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = "create";
      this.error = null;
      try {
        const collection = await createCollection(name);
        await this.load();
        return collection;
      } catch (error) {
        this.error = errorCode(error);
        return null;
      } finally {
        this.workingId = null;
      }
    },
    async rename(collectionId: string, name: string) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_collection_name";
        return null;
      }
      if (this.collections.some((collection) => collection.id !== collectionId && normalizedName(collection.name) === nameKey)) {
        this.error = "duplicate_collection_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = collectionId;
      this.error = null;
      try {
        const collection = await renameCollection(collectionId, name);
        await this.load();
        return collection;
      } catch (error) {
        this.error = errorCode(error);
        return null;
      } finally {
        this.workingId = null;
      }
    },
    async remove(collectionId: string) {
      if (!isTauriEnvironment) return false;

      this.workingId = collectionId;
      this.error = null;
      try {
        await deleteCollection(collectionId);
        this.collections = this.collections.filter((collection) => collection.id !== collectionId);
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async addImage(collectionId: string, imageId: string) {
      if (!isTauriEnvironment) return false;

      this.workingId = collectionId;
      this.error = null;
      try {
        await addImageToCollection(collectionId, imageId);
        await this.load();
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async loadMemberships(collectionIds: string[], imageIds: string[]) {
      if (!isTauriEnvironment || collectionIds.length === 0 || imageIds.length === 0) {
        return [] as CollectionMembership[];
      }

      this.error = null;
      try {
        return await listCollectionMemberships(collectionIds, imageIds);
      } catch (error) {
        this.error = errorCode(error);
        return [] as CollectionMembership[];
      }
    },
    async addImages(collectionIds: string[], imageIds: string[]) {
      if (!isTauriEnvironment || collectionIds.length === 0 || imageIds.length === 0) return false;

      this.workingId = "assign";
      this.error = null;
      try {
        await addImagesToCollections(collectionIds, imageIds);
        await this.load();
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async createAndAdd(name: string, imageIds: string[]) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_collection_name";
        return null;
      }
      if (this.collections.some((collection) => normalizedName(collection.name) === nameKey)) {
        this.error = "duplicate_collection_name";
        return null;
      }
      if (!isTauriEnvironment || imageIds.length === 0) {
        this.error = imageIds.length === 0 ? "empty_image_selection" : null;
        return null;
      }

      this.workingId = "create-and-assign";
      this.error = null;
      try {
        const collection = await createCollectionWithImages(name, imageIds);
        await this.load();
        return collection;
      } catch (error) {
        this.error = errorCode(error);
        return null;
      } finally {
        this.workingId = null;
      }
    },
    async removeImage(collectionId: string, imageId: string) {
      if (!isTauriEnvironment) return false;

      this.workingId = collectionId;
      this.error = null;
      try {
        await removeImageFromCollection(collectionId, imageId);
        await this.load();
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async loadImages(collectionId: string) {
      if (!isTauriEnvironment) return [] as ImageAsset[];

      this.error = null;
      try {
        return await listCollectionImages(collectionId);
      } catch (error) {
        this.error = errorCode(error);
        return [] as ImageAsset[];
      }
    },
  },
});
