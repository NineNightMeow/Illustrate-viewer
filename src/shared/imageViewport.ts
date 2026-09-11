export const MAX_IMAGE_ZOOM = 16;
export const IMAGE_PAN_OVERSCROLL = 32;

export function calculateBaseFitScale(
  surfaceWidth: number,
  surfaceHeight: number,
  imageWidth: number,
  imageHeight: number,
) {
  return Math.min(1, surfaceWidth / imageWidth, surfaceHeight / imageHeight);
}

export function calculateFitZoom(input: {
  surfaceWidth: number;
  surfaceHeight: number;
  imageWidth: number;
  imageHeight: number;
  baseFitScale: number;
  rotation: number;
}) {
  if (input.rotation % 180 === 0) return 1;

  const rotatedFit = calculateBaseFitScale(
    input.surfaceWidth,
    input.surfaceHeight,
    input.imageHeight,
    input.imageWidth,
  );
  return rotatedFit / input.baseFitScale;
}

export function calculateRenderedImageSize(input: {
  imageWidth: number;
  imageHeight: number;
  baseFitScale: number;
  zoom: number;
  rotation: number;
}) {
  const width = input.imageWidth * input.baseFitScale * input.zoom;
  const height = input.imageHeight * input.baseFitScale * input.zoom;

  return input.rotation % 180 === 0 ? { width, height } : { width: height, height: width };
}

export function clampImagePan(input: {
  panX: number;
  panY: number;
  surfaceWidth: number;
  surfaceHeight: number;
  imageWidth: number;
  imageHeight: number;
  overscroll?: number;
}) {
  const overscroll = input.overscroll ?? 0;
  const limitX = Math.max(0, (input.imageWidth - input.surfaceWidth) / 2) + overscroll;
  const limitY = Math.max(0, (input.imageHeight - input.surfaceHeight) / 2) + overscroll;

  return {
    panX: Math.max(-limitX, Math.min(limitX, input.panX)),
    panY: Math.max(-limitY, Math.min(limitY, input.panY)),
  };
}

export function calculateMaximumZoom(baseFitScale: number) {
  return Math.max(MAX_IMAGE_ZOOM, 1 / baseFitScale);
}

export function calculateActualSizeZoom(baseFitScale: number, fitZoom: number) {
  return Math.max(fitZoom, 1 / baseFitScale);
}

export function calculateCursorCenteredZoom(input: {
  currentZoom: number;
  requestedZoom: number;
  minimumZoom: number;
  maximumZoom: number;
  panX: number;
  panY: number;
  clientX: number;
  clientY: number;
  surfaceLeft: number;
  surfaceTop: number;
  surfaceWidth: number;
  surfaceHeight: number;
}) {
  const zoom = Math.max(input.minimumZoom, Math.min(input.maximumZoom, input.requestedZoom));
  if (Math.abs(zoom - input.currentZoom) < 0.001) return null;

  const cursorX = input.clientX - input.surfaceLeft - input.surfaceWidth / 2;
  const cursorY = input.clientY - input.surfaceTop - input.surfaceHeight / 2;
  const ratio = zoom / input.currentZoom;

  return {
    zoom,
    panX: input.panX * ratio + cursorX * (1 - ratio),
    panY: input.panY * ratio + cursorY * (1 - ratio),
  };
}
