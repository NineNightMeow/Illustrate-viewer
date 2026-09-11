import { defineStore } from "pinia";
import {
  addTagToImage,
  createTag,
  deleteTag,
  listImageTags,
  listImageTagsForImages,
  listTags,
  removeTagFromImage,
  renameTag,
  type Tag,
} from "./tagApi";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type TagErrorCode =
  | "invalid_tag_name"
  | "duplicate_tag_name"
  | "tag_not_found"
  | "tag_image_not_found"
  | "tag_persistence_failed";

function parseError(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function errorCode(error: unknown): TagErrorCode {
  const value = typeof error === "string" ? parseError(error) : error;
  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "invalid_tag_name"
      || code === "duplicate_tag_name"
      || code === "tag_not_found"
      || code === "tag_image_not_found"
      || code === "tag_persistence_failed"
    ) return code;
  }
  return "tag_persistence_failed";
}

function normalizedName(name: string) {
  // Keep client-side duplicate checks aligned with SQLite NOCASE (ASCII fold).
  return name.trim().replace(/[A-Z]/g, (character) => character.toLowerCase());
}

export const useTagStore = defineStore("tag", {
  state: () => ({
    tags: [] as Tag[],
    error: null as TagErrorCode | null,
    isLoading: false,
    workingId: null as string | null,
    revision: 0,
    imageTagsByImageId: {} as Record<string, Tag[]>,
  }),
  actions: {
    async load() {
      if (!isTauriEnvironment) return [] as Tag[];

      this.isLoading = true;
      this.error = null;
      try {
        this.tags = await listTags();
        return this.tags;
      } catch (error) {
        this.error = errorCode(error);
        return [] as Tag[];
      } finally {
        this.isLoading = false;
      }
    },
    async create(name: string) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_tag_name";
        return null;
      }
      if (this.tags.some((tag) => normalizedName(tag.name) === nameKey)) {
        this.error = "duplicate_tag_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = "create";
      this.error = null;
      try {
        const tag = await createTag(name);
        this.tags = [...this.tags, tag].sort((left, right) => left.name.localeCompare(right.name));
        this.revision += 1;
        return tag;
      } catch (error) {
        this.error = errorCode(error);
        return null;
      } finally {
        this.workingId = null;
      }
    },
    async rename(tagId: string, name: string) {
      const nameKey = normalizedName(name);
      if (!nameKey) {
        this.error = "invalid_tag_name";
        return null;
      }
      if (this.tags.some((tag) => tag.id !== tagId && normalizedName(tag.name) === nameKey)) {
        this.error = "duplicate_tag_name";
        return null;
      }
      if (!isTauriEnvironment) return null;

      this.workingId = tagId;
      this.error = null;
      try {
        const tag = await renameTag(tagId, name);
        for (const [imageId, imageTags] of Object.entries(this.imageTagsByImageId)) {
          const nextTags = imageTags.map((item) => item.id === tag.id ? tag : item);
          this.imageTagsByImageId = { ...this.imageTagsByImageId, [imageId]: nextTags };
        }
        this.tags = this.tags
          .map((item) => item.id === tag.id ? tag : item)
          .sort((left, right) => left.name.localeCompare(right.name));
        this.revision += 1;
        return tag;
      } catch (error) {
        this.error = errorCode(error);
        return null;
      } finally {
        this.workingId = null;
      }
    },
    async remove(tagId: string) {
      if (!isTauriEnvironment) return false;

      this.workingId = tagId;
      this.error = null;
      try {
        await deleteTag(tagId);
        this.tags = this.tags.filter((tag) => tag.id !== tagId);
        for (const [imageId, imageTags] of Object.entries(this.imageTagsByImageId)) {
          this.imageTagsByImageId = {
            ...this.imageTagsByImageId,
            [imageId]: imageTags.filter((tag) => tag.id !== tagId),
          };
        }
        this.revision += 1;
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async loadImageTags(imageId: string) {
      if (!isTauriEnvironment) return [] as Tag[];

      try {
        const tags = await listImageTags(imageId);
        this.imageTagsByImageId = { ...this.imageTagsByImageId, [imageId]: tags };
        return tags;
      } catch (error) {
        this.error = errorCode(error);
        return [] as Tag[];
      }
    },
    async loadImageTagsForImages(imageIds: string[]) {
      if (!isTauriEnvironment) return {} as Record<string, Tag[]>;
      const ids = [...new Set(imageIds)].filter(Boolean);
      if (ids.length === 0) return {};
      try {
        const result = await listImageTagsForImages(ids);
        this.imageTagsByImageId = { ...this.imageTagsByImageId, ...result };
        for (const imageId of ids) if (!(imageId in result)) this.imageTagsByImageId[imageId] = [];
        return result;
      } catch (error) {
        this.error = errorCode(error);
        return {};
      }
    },
    async addToImages(tagId: string, imageIds: string[]) {
      if (!isTauriEnvironment) return false;

      this.workingId = tagId;
      this.error = null;
      try {
        for (const imageId of new Set(imageIds)) await addTagToImage(tagId, imageId);
        for (const imageId of new Set(imageIds)) {
          const current = this.imageTagsByImageId[imageId] ?? [];
          if (!current.some((tag) => tag.id === tagId)) {
            const tag = this.tags.find((item) => item.id === tagId);
            if (tag) this.imageTagsByImageId = { ...this.imageTagsByImageId, [imageId]: [...current, tag] };
          }
        }
        this.revision += 1;
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
    async removeFromImages(tagId: string, imageIds: string[]) {
      if (!isTauriEnvironment) return false;

      this.workingId = tagId;
      this.error = null;
      try {
        for (const imageId of new Set(imageIds)) await removeTagFromImage(tagId, imageId);
        for (const imageId of new Set(imageIds)) {
          const current = this.imageTagsByImageId[imageId];
          if (current) this.imageTagsByImageId = { ...this.imageTagsByImageId, [imageId]: current.filter((tag) => tag.id !== tagId) };
        }
        this.revision += 1;
        return true;
      } catch (error) {
        this.error = errorCode(error);
        return false;
      } finally {
        this.workingId = null;
      }
    },
  },
});
