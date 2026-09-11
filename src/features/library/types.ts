export type Library = {
  id: string;
  name: string;
  path: string;
  createdAt: number;
  updatedAt: number;
  imageCount: number | null;
  cover: string | null;
};

export type FolderSelection = Pick<Library, "name" | "path">;

export type ImageAsset = {
  id: string;
  libraryId: string;
  path: string;
  filename: string;
  extension: string;
  size: number;
  createdAt: number | null;
  modifiedAt: number;
};

export type ScannerStatus = "idle" | "scanning" | "completed" | "error";

export type ScannerErrorCode =
  | "library_not_found"
  | "library_unavailable"
  | "permission_denied"
  | "scan_failed"
  | "persistence_failed"
  | "scan_in_progress";

export type ScannerState = {
  status: ScannerStatus;
  progress: number | null;
  total: number | null;
  current: number | null;
  error: ScannerErrorCode | null;
};

export type ScanResult = {
  library: Library;
  assets: ImageAsset[];
  skippedEntries: number;
};

export type ThumbnailStatus = "pending" | "generating" | "completed" | "failed";

export type ThumbnailTaskStatus = "queued" | "processing" | "completed" | "failed" | "cancelled";

export type ThumbnailQueueStatus = {
  total: number;
  queued: number;
  queuedHigh: number;
  queuedNormal: number;
  queuedLow: number;
  processing: number;
  completed: number;
  failed: number;
  cancelled: number;
};

export type ThumbnailErrorCode =
  | "library_not_found"
  | "library_unavailable"
  | "source_missing"
  | "source_unreadable"
  | "invalid_source_path"
  | "unsupported_format"
  | "decode_failed"
  | "cache_read_failed"
  | "cache_write_failed"
  | "cancelled"
  | "thumbnail_failed";

export type ThumbnailRecord = {
  imageId: string;
  sourcePath: string;
  sourceModifiedAt: number;
  sourceSize: number;
  thumbnailPath: string | null;
  width: number | null;
  height: number | null;
  generatedAt: number | null;
  status: ThumbnailStatus;
  errorCode: ThumbnailErrorCode | null;
};

export type ThumbnailTaskUpdate = {
  imageId: string;
  status: Extract<ThumbnailStatus, "completed" | "failed">;
  record: ThumbnailRecord | null;
  errorCode: ThumbnailErrorCode | null;
};

export type ThumbnailRequest = Pick<ImageAsset, "id" | "libraryId" | "path">;

export type ThumbnailQueueSubmission = {
  queueStatus: ThumbnailQueueStatus;
  acceptedImageIds: string[];
  cacheHits: ThumbnailRecord[];
};

export type ThumbnailWindow = {
  scopeId: string;
  viewport: ImageAsset[];
  prefetch: ImageAsset[];
};

type LibraryCommandErrorCode =
  | "dialog_failed"
  | "invalid_path"
  | "duplicate_path"
  | "persistence_failed";

export type LibraryCommandError = {
  code: LibraryCommandErrorCode;
  message: string;
};
