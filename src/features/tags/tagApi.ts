import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

export type Tag = {
  id: string;
  name: string;
};

export function createTag(name: string) {
  return invoke<Tag>("create_tag", { name });
}

export function deleteTag(tagId: string) {
  return invoke<void>("delete_tag", { tagId });
}

export function renameTag(tagId: string, name: string) {
  return invoke<Tag>("rename_tag", { tagId, name });
}

export function listTags() {
  return invoke<Tag[]>("list_tags");
}

export function addTagToImage(tagId: string, imageId: string) {
  return invoke<void>("add_tag_to_image", { tagId, imageId });
}

export function removeTagFromImage(tagId: string, imageId: string) {
  return invoke<void>("remove_tag_from_image", { tagId, imageId });
}

export function listImageTags(imageId: string) {
  return invoke<Tag[]>("list_image_tags", { imageId });
}

export function listImageTagsForImages(imageIds: string[]) {
  return invoke<Record<string, Tag[]>>("list_image_tags_for_images", { imageIds });
}

export function listTagImages(tagId: string) {
  return invoke<ImageAsset[]>("list_tag_images", { tagId });
}
