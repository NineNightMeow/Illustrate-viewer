import { defineStore } from "pinia";
import type { ImageAsset } from "../library/types";
import {
  createSmartCollection,
  deleteSmartCollection,
  listSmartCollectionImages,
  listSmartCollections,
  renameSmartCollection,
  type SmartCollection,
  type SmartCollectionRule,
} from "./smartCollectionApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type SmartCollectionSummary = SmartCollection & { imageCount: number };
export type SmartCollectionErrorCode =
  | "invalid_smart_collection_name"
  | "duplicate_smart_collection_name"
  | "invalid_smart_collection_rule"
  | "smart_collection_not_found"
  | "smart_collection_persistence_failed";

function errorCode(error: unknown): SmartCollectionErrorCode {
  const value = typeof error === "string" ? safelyParse(error) : error;
  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "invalid_smart_collection_name"
      || code === "duplicate_smart_collection_name"
      || code === "invalid_smart_collection_rule"
      || code === "smart_collection_not_found"
      || code === "smart_collection_persistence_failed"
    ) return code;
  }
  return "smart_collection_persistence_failed";
}

function safelyParse(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function normalizedName(name: string) {
  return name.trim().toLocaleLowerCase();
}

function sortCollections(collections: SmartCollectionSummary[]) {
  return [...collections].sort((left, right) => right.updatedAt - left.updatedAt);
}

export const useSmartCollectionStore = defineStore("smartCollection", {
  state: () => ({
    collections: [] as SmartCollectionSummary[],
    error: null as SmartCollectionErrorCode | null,
    isLoading: false,
    workingId: null as string | null,
  }),
  actions: {
    async load() {
      if (!isTauriEnvironment) return [] as SmartCollectionSummary[];

      this.isLoading = true;
      this.error = null;
      try {
        const collections = await listSmartCollections();
        const summaries = await Promise.all(collections.map(async (collection) => ({
          ...collection,
          imageCount: (await listSmartCollectionImages(collection.id)).length,
        })));
        this.collections = sortCollections(summaries);
        return this.collections;
      } catch (error) {
        this.error = errorCode(error);
        return [] as SmartCollectionSummary[];
      } finally {
        this.isLoading = false;
      }
    },
    async create(name: string, rule: SmartCollectionRule) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_smart_collection_name";
        return null;
      }
      if (this.collections.some((collection) => normalizedName(collection.name) === nameKey)) {
        this.error = "duplicate_smart_collection_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = "create";
      this.error = null;
      try {
        const collection = await createSmartCollection(name, rule);
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
        this.error = "invalid_smart_collection_name";
        return null;
      }
      if (this.collections.some((collection) => collection.id !== collectionId && normalizedName(collection.name) === nameKey)) {
        this.error = "duplicate_smart_collection_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = collectionId;
      this.error = null;
      try {
        const collection = await renameSmartCollection(collectionId, name);
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
        await deleteSmartCollection(collectionId);
        this.collections = this.collections.filter((collection) => collection.id !== collectionId);
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
        return await listSmartCollectionImages(collectionId);
      } catch (error) {
        this.error = errorCode(error);
        return [] as ImageAsset[];
      }
    },
  },
});
