import { invoke } from "@tauri-apps/api/core";

export type ImageMetadata = {
  imageId: string;
  width: number | null;
  height: number | null;
  aspectRatio: number | null;
  fileSize: number | null;
  format: string | null;
  hasAlpha: boolean | null;
  cameraMake: string | null;
  cameraModel: string | null;
  capturedAt: string | null;
  orientation: number | null;
  extractionVersion: number;
  status: "ready" | "failed";
  failureReason: string | null;
  extractedAt: number;
  sourceCreatedAt: number | null;
  sourceModifiedAt: number;
};

export type MetadataTaskStatus = "idle" | "running" | "completed" | "failed" | "cancelled";

export type MetadataTaskUpdate = {
  imageId: string;
  status: MetadataTaskStatus;
  metadata: ImageMetadata | null;
  errorCode: string | null;
};

export type MetadataQueueSubmission = {
  acceptedImageIds: string[];
};

export function getImageMetadata(imageId: string) {
  return invoke<ImageMetadata | null>("get_image_metadata", { imageId });
}

export function enqueueMetadataTasks(imageIds: string[]) {
  return invoke<MetadataQueueSubmission>("enqueue_metadata_tasks", { imageIds });
}

export function retryImageMetadata(imageId: string) {
  return invoke<MetadataQueueSubmission>("retry_image_metadata", { imageId });
}
