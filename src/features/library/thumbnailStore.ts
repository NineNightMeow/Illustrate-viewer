import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { shallowReactive } from "vue";
import {
  cancelThumbnailTasks as cancelThumbnailTasksRequest,
  clearThumbnailCache as clearThumbnailCacheRequest,
  enqueueThumbnailTasks as enqueueThumbnailTasksRequest,
  getThumbnailQueueStatus,
  releaseThumbnailWindow as releaseThumbnailWindowRequest,
  syncThumbnailWindow as syncThumbnailWindowRequest,
} from "./thumbnailApi";
import type {
  ThumbnailWindow,
  ImageAsset,
  ThumbnailErrorCode,
  ThumbnailQueueStatus,
  ThumbnailQueueSubmission,
  ThumbnailRecord,
  ThumbnailTaskUpdate,
} from "./types";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let queueStatusTimer: ReturnType<typeof setInterval> | null = null;
let taskUpdatesUnlisten: UnlistenFn | null = null;
let taskUpdatesPromise: Promise<void> | null = null;
let pendingThumbnailWindow: ThumbnailWindow | null = null;
let thumbnailWindowSyncPromise: Promise<ThumbnailQueueStatus | null> | null = null;

function hasActiveTasks(status: ThumbnailQueueStatus) {
  return status.queued > 0 || status.processing > 0;
}

function parseError(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function errorCode(error: unknown): ThumbnailErrorCode {
  const value = typeof error === "string" ? parseError(error) : error;

  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "library_not_found" ||
      code === "library_unavailable" ||
      code === "source_missing" ||
      code === "source_unreadable" ||
      code === "invalid_source_path" ||
      code === "unsupported_format" ||
      code === "decode_failed" ||
      code === "cache_read_failed" ||
      code === "cache_write_failed" ||
      code === "cancelled" ||
      code === "thumbnail_failed"
    ) {
      return code;
    }
  }

  return "thumbnail_failed";
}

function recordFor(
  asset: ImageAsset,
  status: ThumbnailRecord["status"],
  errorCode: ThumbnailErrorCode | null = null,
): ThumbnailRecord {
  return {
    imageId: asset.id,
    sourcePath: asset.path,
    sourceModifiedAt: asset.modifiedAt,
    sourceSize: asset.size,
    thumbnailPath: null,
    width: null,
    height: null,
    generatedAt: null,
    status,
    errorCode,
  };
}

function needsThumbnail(asset: ImageAsset, records: Record<string, ThumbnailRecord>) {
  const record = records[asset.id];
  return (
    !record ||
    record.sourceModifiedAt !== asset.modifiedAt ||
    record.sourceSize !== asset.size ||
    record.status === "pending" ||
    record.status === "generating" ||
    (record.status === "failed" && record.errorCode === "cancelled")
  );
}

function uniqueAssets(assets: ImageAsset[]) {
  const unique = new Map<string, ImageAsset>();
  for (const asset of assets) unique.set(asset.id, asset);
  return [...unique.values()];
}

export const useThumbnailStore = defineStore("thumbnail", {
  state: () => ({
    recordsByImageId: shallowReactive({}) as Record<string, ThumbnailRecord>,
    cacheError: null as ThumbnailErrorCode | null,
    isClearingCache: false,
    queueStatus: null as ThumbnailQueueStatus | null,
    queueError: null as ThumbnailErrorCode | null,
  }),
  actions: {
    async startTaskUpdates() {
      if (!isTauriEnvironment || taskUpdatesUnlisten !== null) return;
      if (taskUpdatesPromise === null) {
        taskUpdatesPromise = listen<ThumbnailTaskUpdate>("thumbnail-task-updated", ({ payload }) => {
          if (payload.status === "completed" && payload.record) {
            this.recordsByImageId[payload.imageId] = payload.record;
            return;
          }

          const current = this.recordsByImageId[payload.imageId];
          if (current) {
            this.recordsByImageId[payload.imageId] = {
              ...current,
              status: "failed",
              errorCode: payload.errorCode ?? "thumbnail_failed",
            };
          }
        })
          .then((unlisten) => {
            taskUpdatesUnlisten = unlisten;
          })
          .catch((error) => {
            taskUpdatesPromise = null;
            throw error;
          });
      }
      await taskUpdatesPromise;
    },
    markPending(assets: ImageAsset[]) {
      for (const asset of assets) {
        const current = this.recordsByImageId[asset.id];
        if (!current || current.sourceModifiedAt !== asset.modifiedAt || current.sourceSize !== asset.size) {
          this.recordsByImageId[asset.id] = recordFor(asset, "pending");
        }
      }
    },
    applySubmission(submission: ThumbnailQueueSubmission, requested: ImageAsset[]) {
      const cacheHitIds = new Set<string>();
      for (const record of submission.cacheHits) {
        this.recordsByImageId[record.imageId] = record;
        cacheHitIds.add(record.imageId);
      }

      const accepted = new Set(submission.acceptedImageIds);
      for (const asset of requested) {
        if (cacheHitIds.has(asset.id) || !accepted.has(asset.id)) continue;
        const current = this.recordsByImageId[asset.id] ?? recordFor(asset, "pending");
        this.recordsByImageId[asset.id] = {
          ...current,
          status: "generating",
          errorCode: null,
        };
      }
      this.updateQueueStatus(submission.queueStatus);
    },
    async ensureThumbnails(assets: ImageAsset[]) {
      if (!isTauriEnvironment || assets.length === 0) return null;
      const requested = uniqueAssets(assets.filter((asset) => needsThumbnail(asset, this.recordsByImageId)));
      if (requested.length === 0) return this.queueStatus;

      try {
        await this.startTaskUpdates();
        this.markPending(requested);
        const submission = await enqueueThumbnailTasksRequest(requested);
        this.queueError = null;
        this.applySubmission(submission, requested);
        return submission.queueStatus;
      } catch (error) {
        this.queueError = errorCode(error);
        return null;
      }
    },
    async syncThumbnailWindow(scopeId: string, viewport: ImageAsset[], prefetch: ImageAsset[]) {
      if (!isTauriEnvironment || !scopeId) return null;
      pendingThumbnailWindow = { scopeId, viewport, prefetch };
      if (thumbnailWindowSyncPromise) return thumbnailWindowSyncPromise;

      thumbnailWindowSyncPromise = (async () => {
        let latestStatus: ThumbnailQueueStatus | null = null;
        while (pendingThumbnailWindow) {
          const window = pendingThumbnailWindow;
          pendingThumbnailWindow = null;
          latestStatus = await this.submitThumbnailWindow(window);
        }
        return latestStatus;
      })();
      try {
        return await thumbnailWindowSyncPromise;
      } finally {
        thumbnailWindowSyncPromise = null;
      }
    },
    async submitThumbnailWindow(window: ThumbnailWindow) {
      const viewport = uniqueAssets(
        window.viewport.filter((asset) => needsThumbnail(asset, this.recordsByImageId)),
      );
      const viewportIds = new Set(viewport.map((asset) => asset.id));
      const prefetch = uniqueAssets(
        window.prefetch.filter(
          (asset) => !viewportIds.has(asset.id) && needsThumbnail(asset, this.recordsByImageId),
        ),
      );
      const requested = [...viewport, ...prefetch];

      try {
        if (requested.length > 0) {
          await this.startTaskUpdates();
          this.markPending(requested);
        }
        const submission = await syncThumbnailWindowRequest({
          scopeId: window.scopeId,
          viewport,
          prefetch,
        });
        this.queueError = null;
        this.applySubmission(submission, requested);
        return submission.queueStatus;
      } catch (error) {
        this.queueError = errorCode(error);
        return null;
      }
    },
    async releaseThumbnailWindow(scopeId: string) {
      if (!isTauriEnvironment || !scopeId) return null;
      pendingThumbnailWindow = null;
      try {
        await thumbnailWindowSyncPromise;
        const status = await releaseThumbnailWindowRequest(scopeId);
        this.queueError = null;
        this.updateQueueStatus(status);
        return status;
      } catch (error) {
        this.queueError = errorCode(error);
        return null;
      }
    },
    async generateThumbnail(asset: ImageAsset) {
      return this.ensureThumbnails([asset]);
    },
    async clearThumbnailCache() {
      if (!isTauriEnvironment) return false;

      this.isClearingCache = true;
      this.cacheError = null;

      try {
        await clearThumbnailCacheRequest();
        for (const imageId of Object.keys(this.recordsByImageId)) {
          delete this.recordsByImageId[imageId];
        }
        return true;
      } catch (error) {
        this.cacheError = errorCode(error);
        return false;
      } finally {
        this.isClearingCache = false;
      }
    },
    updateQueueStatus(status: ThumbnailQueueStatus) {
      this.queueStatus = status;
      if (hasActiveTasks(status)) {
        this.startQueueStatusPolling();
      } else {
        this.stopQueueStatusPolling();
      }
    },
    startQueueStatusPolling() {
      if (!isTauriEnvironment || queueStatusTimer !== null) return;

      queueStatusTimer = window.setInterval(() => {
        void this.refreshThumbnailQueueStatus();
      }, 500);
    },
    stopQueueStatusPolling() {
      if (queueStatusTimer === null) return;

      window.clearInterval(queueStatusTimer);
      queueStatusTimer = null;
    },
    async refreshThumbnailQueueStatus() {
      if (!isTauriEnvironment) return null;

      try {
        const status = await getThumbnailQueueStatus();
        this.queueError = null;
        this.updateQueueStatus(status);
        return status;
      } catch (error) {
        this.queueError = errorCode(error);
        this.stopQueueStatusPolling();
        return null;
      }
    },
    async cancelThumbnailTasks(libraryId?: string) {
      if (!isTauriEnvironment) return null;

      try {
        const status = await cancelThumbnailTasksRequest(libraryId);
        this.queueError = null;
        if (!libraryId) {
          for (const [imageId, record] of Object.entries(this.recordsByImageId)) {
            if (record.status === "pending" || record.status === "generating") {
              this.recordsByImageId[imageId] = {
                ...record,
                status: "failed",
                errorCode: "cancelled",
              };
            }
          }
        }
        this.updateQueueStatus(status);
        return status;
      } catch (error) {
        this.queueError = errorCode(error);
        return null;
      }
    },
  },
});
