import { isEditableTarget } from "../../shared/shortcuts/shortcutRegistry.ts";

export const MIN_THUMBNAIL_SIZE = 120;
export const MAX_THUMBNAIL_SIZE = 320;
export const THUMBNAIL_SIZE_STEP = 20;
const DEFAULT_THUMBNAIL_SIZE = 160;
const GRID_GAP = 12;
const GRID_PADDING = 16;
const ITEM_META_HEIGHT = 54;
const ROW_OVERSCAN = 2;

export type VirtualGridLayout = {
  columns: number;
  itemCount: number;
  rowHeight: number;
  rowCount: number;
  totalHeight: number;
};

export function clampThumbnailSize(value: number) {
  return Math.min(MAX_THUMBNAIL_SIZE, Math.max(MIN_THUMBNAIL_SIZE, Math.round(value)));
}

export function adjustThumbnailSize(current: number, deltaY: number) {
  if (deltaY === 0) return clampThumbnailSize(current);
  const direction = deltaY < 0 ? 1 : -1;
  return clampThumbnailSize(current + direction * THUMBNAIL_SIZE_STEP);
}

export function shouldResizeThumbnails(event: Pick<WheelEvent, "ctrlKey" | "deltaY" | "target">) {
  return event.ctrlKey && event.deltaY !== 0 && !isEditableTarget(event.target);
}

export function createVirtualGridLayout(
  viewportWidth: number,
  itemCount: number,
  thumbnailSize = DEFAULT_THUMBNAIL_SIZE,
): VirtualGridLayout {
  const safeItemCount = Math.max(0, itemCount);
  const contentWidth = Math.max(1, viewportWidth - GRID_PADDING * 2);
  const columns = Math.max(1, Math.floor((contentWidth + GRID_GAP) / (clampThumbnailSize(thumbnailSize) + GRID_GAP)));
  const itemWidth = (contentWidth - (columns - 1) * GRID_GAP) / columns;
  const rowHeight = itemWidth + ITEM_META_HEIGHT + GRID_GAP;
  const rowCount = Math.ceil(safeItemCount / columns);

  return {
    columns,
    itemCount: safeItemCount,
    rowHeight,
    rowCount,
    totalHeight: Math.max(0, rowCount * rowHeight - GRID_GAP),
  };
}

export function createVirtualGridRange(
  layout: VirtualGridLayout,
  scrollTop: number,
  viewportHeight: number,
  overscanRows = ROW_OVERSCAN,
) {
  const maxScrollTop = Math.max(0, layout.totalHeight - viewportHeight);
  const safeScrollTop = Math.min(Math.max(0, scrollTop), maxScrollTop);
  const startRow = Math.max(0, Math.floor(safeScrollTop / layout.rowHeight) - overscanRows);
  const endRow = Math.min(
    layout.rowCount,
    Math.ceil((safeScrollTop + Math.max(viewportHeight, layout.rowHeight)) / layout.rowHeight) + overscanRows,
  );

  return {
    startIndex: startRow * layout.columns,
    endIndex: Math.min(layout.itemCount, endRow * layout.columns),
    offset: startRow * layout.rowHeight,
  };
}
