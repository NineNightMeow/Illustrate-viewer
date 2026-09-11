import { invoke } from "@tauri-apps/api/core";
import type {
  ThumbnailWindow,
  ThumbnailQueueStatus,
  ThumbnailQueueSubmission,
  ThumbnailRequest,
} from "./types";

export function clearThumbnailCache() {
  return invoke<void>("clear_thumbnail_cache");
}

export function enqueueThumbnailTasks(tasks: ThumbnailRequest[]) {
  return invoke<ThumbnailQueueSubmission>("enqueue_thumbnail_tasks", {
    tasks: serializeTasks(tasks),
  });
}

export function syncThumbnailWindow(window: ThumbnailWindow) {
  return invoke<ThumbnailQueueSubmission>("sync_gallery_thumbnail_window", {
    window: {
      scopeId: window.scopeId,
      viewport: serializeTasks(window.viewport),
      prefetch: serializeTasks(window.prefetch),
    },
  });
}

export function releaseThumbnailWindow(scopeId: string) {
  return invoke<ThumbnailQueueStatus>("release_gallery_thumbnail_window", { scopeId });
}

export function getThumbnailQueueStatus() {
  return invoke<ThumbnailQueueStatus>("get_thumbnail_queue_status");
}

export function cancelThumbnailTasks(libraryId?: string) {
  return invoke<ThumbnailQueueStatus>("cancel_thumbnail_tasks", { libraryId });
}

function serializeTasks(tasks: ThumbnailRequest[]) {
  return tasks.map((task) => ({
    imageId: task.id,
    libraryId: task.libraryId,
    sourcePath: task.path,
  }));
}
