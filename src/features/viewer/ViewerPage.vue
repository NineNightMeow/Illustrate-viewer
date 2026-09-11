<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import {
  ArrowLeft,
  Bookmark,
  ChevronLeft,
  ChevronRight,
  Ellipsis,
  FolderPlus,
  Heart,
  Info,
  RotateCw,
  Tags,
} from "lucide-vue-next";
import AppTooltip from "../../shared/components/ui/AppTooltip.vue";
import {
  IMAGE_PAN_OVERSCROLL,
  MAX_IMAGE_ZOOM,
  calculateActualSizeZoom,
  calculateBaseFitScale,
  calculateCursorCenteredZoom,
  calculateFitZoom,
  calculateRenderedImageSize,
  clampImagePan,
} from "../../shared/imageViewport";
import { useLibraryStore } from "../library/libraryStore";
import { useThumbnailStore } from "../library/thumbnailStore";
import { openReferenceWindow } from "../reference/referenceWindowApi";
import {
  analyzeImageColors,
  getImageColorMetadata,
  sampleImageColor,
  type Color,
  type ImageColorMetadata,
} from "../color/colorApi";
import ImageInformationPanel from "../color/ImageInformationPanel.vue";
import {
  enqueueMetadataTasks,
  getImageMetadata,
  retryImageMetadata,
  type ImageMetadata,
  type MetadataTaskStatus,
  type MetadataTaskUpdate,
} from "../metadata/metadataApi";
import DuplicateCandidateDialog from "../fingerprint/DuplicateCandidateDialog.vue";
import {
  listDuplicateCandidates,
  type DuplicateCandidate,
  type DuplicateCandidatePage,
} from "../fingerprint/fingerprintApi";
import { useViewerStore, type ViewerBackground } from "./viewerStore";
import AddToCollectionDialog from "../collections/AddToCollectionDialog.vue";
import TagAssignmentDialog from "../tags/TagAssignmentDialog.vue";
import { listImageTags } from "../tags/tagApi";
import { useTagStore } from "../tags/tagStore";
import { shortcutDispatcher } from "../../shared/shortcuts/shortcutDispatcher";
import { getViewerEscapeLayer } from "./viewerEscLayer";

const COLOR_MAGNIFIER_RADIUS = 67;
const COLOR_MAGNIFIER_PIXEL_SIZE = 20;
const COLOR_MAGNIFIER_OFFSET = 20;

type ColorMagnifier = {
  x: number;
  y: number;
  left: number;
  top: number;
};

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const libraryStore = useLibraryStore();
const thumbnailStore = useThumbnailStore();
const viewerStore = useViewerStore();
const tagStore = useTagStore();
const imageFailed = ref(false);
const previewFailed = ref(false);
const isFullImageLoaded = ref(false);
const controlsVisible = ref(true);
const backgroundMenuOpen = ref(false);
const collectionDialogOpen = ref(false);
const tagDialogOpen = ref(false);
const duplicateDialogOpen = ref(false);
const informationPanelOpen = ref(false);
const imageMetadata = ref<ImageMetadata | null>(null);
const metadataTaskStatus = ref<MetadataTaskStatus>("idle");
const hasMetadataError = ref(false);
const colorMetadata = ref<ImageColorMetadata | null>(null);
const selectedColor = ref<Color | null>(null);
const pickedColor = ref<{ color: Color; x: number; y: number } | null>(null);
const isColorAnalyzing = ref(false);
const hasColorAnalysisError = ref(false);
const colorPickerActive = ref(false);
const isColorPicking = ref(false);
const colorMagnifier = ref<ColorMagnifier | null>(null);
const duplicateSummary = ref<DuplicateCandidatePage | null>(null);
const duplicateCandidates = ref<DuplicateCandidate[]>([]);
const isDuplicateSummaryLoading = ref(false);
const isDuplicateListLoading = ref(false);
const hasDuplicateSummaryError = ref(false);
const hasDuplicateListError = ref(false);
const duplicateCandidateImageId = ref<string | null>(null);
const imageTags = ref<{ id: string; name: string }[]>([]);
const surface = ref<HTMLElement | null>(null);
const imageElement = ref<HTMLImageElement | null>(null);
const baseFitScale = ref<number | null>(null);
const isDragging = ref(false);
const zoomHudVisible = ref(false);
const libraryId = computed(() => typeof route.params.libraryId === "string" ? route.params.libraryId : "");
const hasSession = computed(() =>
  viewerStore.entryLibraryId === libraryId.value
  && viewerStore.currentImageId !== null
  && viewerStore.resultImageIds.length > 0,
);
const currentLibraryId = computed(() => viewerStore.libraryIdFor(viewerStore.currentImageId));
const currentLibraryAssets = computed(() => currentLibraryId.value
  ? libraryStore.assetsByLibrary[currentLibraryId.value] ?? []
  : []);
const isCurrentLibraryResolved = ref(false);
const currentAsset = computed(() => findAsset(viewerStore.currentImageId));
const currentIndex = computed(() => viewerStore.currentIndex);
const previousImageId = computed(() => currentIndex.value > 0
  ? viewerStore.resultImageIds[currentIndex.value - 1] ?? null
  : null);
const nextImageId = computed(() => currentIndex.value >= 0 && currentIndex.value < viewerStore.resultImageIds.length - 1
  ? viewerStore.resultImageIds[currentIndex.value + 1] ?? null
  : null);
const previousAsset = computed(() => findAsset(previousImageId.value));
const nextAsset = computed(() => findAsset(nextImageId.value));
const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const imageSource = computed(() => {
  if (!viewerStore.sourcePath) return null;
  return isTauriEnvironment ? convertFileSrc(viewerStore.sourcePath) : viewerStore.sourcePath;
});
const thumbnailPreviewSource = computed(() => {
  const imageId = currentAsset.value?.id;
  const record = imageId ? thumbnailStore.recordsByImageId[imageId] : null;

  if (!isTauriEnvironment || !record?.thumbnailPath || record.status !== "completed") return null;
  return convertFileSrc(record.thumbnailPath);
});
const hasCurrentAssetList = computed(() => Boolean(
  currentLibraryId.value && currentLibraryId.value in libraryStore.assetsByLibrary,
));
const isCurrentLibraryScanning = computed(() =>
  currentLibraryId.value
  && libraryStore.scannerStates[currentLibraryId.value]?.status === "scanning",
);
const currentLibraryResolutionFailed = computed(() => Boolean(
  currentLibraryId.value
  && isCurrentLibraryResolved.value
  && !hasCurrentAssetList.value
  && !isCurrentLibraryScanning.value,
));
const isResolvingAsset = computed(() => hasSession.value
  && !currentAsset.value
  && !hasCurrentAssetList.value
  && !currentLibraryResolutionFailed.value,
);
const showError = computed(() =>
  viewerStore.hasError
  || imageFailed.value
  || (hasSession.value && (!currentLibraryId.value
    || currentLibraryResolutionFailed.value
    || (hasCurrentAssetList.value && !currentAsset.value))),
);
const errorMessage = computed(() => {
  if (imageFailed.value) return t("viewer.sourceUnreadable");
  switch (viewerStore.errorCode) {
    case "source_missing": return t("viewer.sourceMissing");
    case "source_unreadable":
    case "decode_failed": return t("viewer.sourceUnreadable");
    case "unsupported_format": return t("viewer.unsupportedFormat");
    case "library_unavailable":
    case "library_not_found": return t("viewer.libraryUnavailable");
    default: return t("viewer.unavailable");
  }
});
const backgroundClass = computed(() => `viewer-page--background-${viewerStore.background}`);
const fitZoom = computed(() => getFitZoom());
const maximumZoom = computed(() => Math.max(fitZoom.value, MAX_IMAGE_ZOOM));
const canAnalyzeColors = computed(() => Boolean(currentAsset.value) && isTauriEnvironment);
const canLoadMetadata = computed(() => Boolean(currentAsset.value) && isTauriEnvironment);
const canPickColor = computed(() => Boolean(
  currentAsset.value
  && isFullImageLoaded.value
  && imageElement.value?.naturalWidth
  && baseFitScale.value !== null
  && isTauriEnvironment
  && !isColorPicking.value,
));
const canViewDuplicates = computed(() => Boolean(currentAsset.value) && isTauriEnvironment);
const zoomLabel = computed(() => baseFitScale.value === null
  ? t("viewer.fit")
  : `${Math.max(1, Math.round(baseFitScale.value * viewerStore.zoom * 100))}%`);
const imageTransform = computed(() =>
  `translate3d(${viewerStore.panX}px, ${viewerStore.panY}px, 0) rotate(${viewerStore.rotation}deg) scale(${viewerStore.zoom})`,
);
const imageStyle = computed(() => {
  const image = imageElement.value;
  const fitScale = baseFitScale.value;

  if (!image?.naturalWidth || !image.naturalHeight || fitScale === null) {
    return { transform: imageTransform.value };
  }

  return {
    width: `${image.naturalWidth * fitScale}px`,
    height: `${image.naturalHeight * fitScale}px`,
    transform: imageTransform.value,
  };
});
const colorMagnifierStyle = computed(() => {
  const magnifier = colorMagnifier.value;
  const image = imageElement.value;
  const source = imageSource.value;
  if (!magnifier || !image?.naturalWidth || !image.naturalHeight || !source) return {};

  const diameter = COLOR_MAGNIFIER_RADIUS * 2;
  const gridOffset = COLOR_MAGNIFIER_RADIUS - COLOR_MAGNIFIER_PIXEL_SIZE / 2;
  return {
    left: `${magnifier.left}px`,
    top: `${magnifier.top}px`,
    width: `${diameter}px`,
    height: `${diameter}px`,
    backgroundImage: `url("${source}")`,
    backgroundPosition: `${COLOR_MAGNIFIER_RADIUS - (magnifier.x + 0.5) * COLOR_MAGNIFIER_PIXEL_SIZE}px ${COLOR_MAGNIFIER_RADIUS - (magnifier.y + 0.5) * COLOR_MAGNIFIER_PIXEL_SIZE}px`,
    backgroundSize: `${image.naturalWidth * COLOR_MAGNIFIER_PIXEL_SIZE}px ${image.naturalHeight * COLOR_MAGNIFIER_PIXEL_SIZE}px`,
    "--iv-color-magnifier-pixel-size": `${COLOR_MAGNIFIER_PIXEL_SIZE}px`,
    "--iv-color-magnifier-grid-offset": `${gridOffset}px`,
  };
});
const backgrounds: ViewerBackground[] = ["auto", "dark", "light"];
let controlsTimer: ReturnType<typeof setTimeout> | null = null;
let wheelTimestamp = 0;
let prefetchSequence = 0;
let prefetchImages: HTMLImageElement[] = [];
let resizeObserver: ResizeObserver | null = null;
let zoomHudTimer: ReturnType<typeof setTimeout> | null = null;
let unregisterShortcuts: (() => void) | null = null;
let colorMetadataRequest = 0;
let imageMetadataRequest = 0;
let colorPickRequest = 0;
let duplicateSummaryRequest = 0;
let duplicateCandidateRequest = 0;
let metadataTaskUpdatesUnlisten: UnlistenFn | null = null;
let metadataTaskUpdatesActive = true;
let dragState: {
  pointerId: number;
  startX: number;
  startY: number;
  panX: number;
  panY: number;
} | null = null;

watch(currentAsset, (asset) => {
  imageFailed.value = false;
  previewFailed.value = false;
  isFullImageLoaded.value = false;
  baseFitScale.value = null;
  if (asset && hasSession.value) void viewerStore.prepareCurrent(asset);
}, { immediate: true });

watch([currentIndex, currentLibraryAssets, previousAsset, nextAsset, () => viewerStore.sourceImageId], () => {
  if (viewerStore.sourceImageId === viewerStore.currentImageId) {
    schedulePrefetch();
    return;
  }

  prefetchSequence += 1;
  clearPrefetch();
}, { immediate: true });

watch(
  currentLibraryId,
  async (id) => {
    isCurrentLibraryResolved.value = false;
    if (!id || !hasSession.value) return;

    await libraryStore.ensureLibraryAssets(id);
    if (id === currentLibraryId.value) isCurrentLibraryResolved.value = true;
  },
  { immediate: true },
);

watch(imageSource, () => {
  imageFailed.value = false;
  isFullImageLoaded.value = false;
  colorPickerActive.value = false;
  isColorPicking.value = false;
  colorMagnifier.value = null;
  colorPickRequest += 1;
});

watch(() => currentAsset.value?.id, (imageId) => {
  const request = ++colorMetadataRequest;
  colorMetadata.value = null;
  selectedColor.value = null;
  pickedColor.value = null;
  hasColorAnalysisError.value = false;
  colorPickerActive.value = false;
  isColorPicking.value = false;
  colorMagnifier.value = null;
  colorPickRequest += 1;
  duplicateSummaryRequest += 1;
  duplicateCandidateRequest += 1;
  duplicateSummary.value = null;
  duplicateCandidates.value = [];
  hasDuplicateSummaryError.value = false;
  hasDuplicateListError.value = false;
  isDuplicateSummaryLoading.value = false;
  isDuplicateListLoading.value = false;
  duplicateCandidateImageId.value = null;
  duplicateDialogOpen.value = false;
  if (!imageId || !isTauriEnvironment) return;

  void getImageColorMetadata(imageId).then((metadata) => {
    if (request !== colorMetadataRequest) return;
    colorMetadata.value = metadata;
    selectedColor.value = metadata?.dominantColors[0] ?? null;
  }).catch(() => {
    if (request !== colorMetadataRequest) return;
    hasColorAnalysisError.value = true;
  });
}, { immediate: true });

watch(() => currentAsset.value?.id, (imageId) => {
  imageTags.value = [];
  if (!imageId || !isTauriEnvironment) return;
  void listImageTags(imageId).then((tags) => {
    if (currentAsset.value?.id === imageId) {
      imageTags.value = tags;
      tagStore.imageTagsByImageId = { ...tagStore.imageTagsByImageId, [imageId]: tags };
    }
  }).catch(() => undefined);
}, { immediate: true });

watch(() => currentAsset.value?.id, (imageId) => {
  const request = ++imageMetadataRequest;
  imageMetadata.value = null;
  metadataTaskStatus.value = "idle";
  hasMetadataError.value = false;
  if (!imageId || !isTauriEnvironment) {
    metadataTaskStatus.value = "completed";
    return;
  }
  void loadImageMetadata(imageId, request);
}, { immediate: true });

watch([informationPanelOpen, () => currentAsset.value?.id], ([open, imageId]) => {
  if (open && imageId && !duplicateSummary.value && !isDuplicateSummaryLoading.value) {
    void loadDuplicateSummary(imageId);
  }
});

watch(duplicateCandidates, (candidates) => {
  const source = currentAsset.value;
  if (!source || candidates.length === 0) return;
  void thumbnailStore.ensureThumbnails([source, ...candidates.map((candidate) => candidate.asset)]);
});

onMounted(() => {
  if (hasSession.value && currentLibraryId.value) void libraryStore.ensureLibraryAssets(currentLibraryId.value);
  unregisterShortcuts = shortcutDispatcher.register("viewer", {
    "viewer.escape": () => {
      const escapeLayer = getViewerEscapeLayer({
        colorPickerOpen: colorPickerActive.value,
        informationPanelOpen: informationPanelOpen.value,
        backgroundMenuOpen: backgroundMenuOpen.value,
      });
      if (escapeLayer === "colorPicker") {
        colorPickerActive.value = false;
        clearColorMagnifier();
        return;
      }
      if (escapeLayer === "informationPanel") {
        informationPanelOpen.value = false;
        return;
      }
      if (escapeLayer === "backgroundMenu") {
        backgroundMenuOpen.value = false;
        return;
      }
      void exitFullscreen().then((exited) => {
        if (!exited) returnToGallery();
      });
    },
    "viewer.fullscreen": () => { void toggleFullscreen(); },
    "viewer.reference": () => {
      if (currentAsset.value) openCurrentReference();
      else return false;
    },
    "viewer.fit": () => { resetFit(); },
    "viewer.actualSize": () => { setActualSize(); },
    "viewer.previous": () => { navigate(-1); },
    "viewer.next": () => { navigate(1); },
  });
  resizeObserver = new ResizeObserver(updateFitPercentage);
  if (surface.value) resizeObserver.observe(surface.value);
  revealControls();
});

void startMetadataTaskUpdates();

onBeforeUnmount(() => {
  unregisterShortcuts?.();
  resizeObserver?.disconnect();
  if (controlsTimer !== null) window.clearTimeout(controlsTimer);
  if (zoomHudTimer !== null) window.clearTimeout(zoomHudTimer);
  prefetchSequence += 1;
  clearPrefetch();
  metadataTaskUpdatesActive = false;
  metadataTaskUpdatesUnlisten?.();
  metadataTaskUpdatesUnlisten = null;
});

function findAsset(imageId: string | null | undefined) {
  const assetLibraryId = viewerStore.libraryIdFor(imageId);
  if (!imageId || !assetLibraryId) return null;
  return libraryStore.assetsByLibrary[assetLibraryId]?.find((asset) => asset.id === imageId) ?? null;
}

function schedulePrefetch() {
  const request = ++prefetchSequence;
  clearPrefetch();

  for (const asset of [previousAsset.value, nextAsset.value]) {
    if (!asset) continue;
    void viewerStore.preparePrefetch(asset).then((sourcePath) => {
      if (!sourcePath || request !== prefetchSequence) return;

      const image = new Image();
      image.decoding = "async";
      image.src = isTauriEnvironment ? convertFileSrc(sourcePath) : sourcePath;
      prefetchImages.push(image);
    });
  }
}

function clearPrefetch() {
  for (const image of prefetchImages) image.src = "";
  prefetchImages = [];
}

function handleFullImageLoad() {
  isFullImageLoaded.value = true;
  updateFitPercentage();
}

async function startMetadataTaskUpdates() {
  if (!isTauriEnvironment) return;
  const unlisten = await listen<MetadataTaskUpdate>("metadata-task-updated", ({ payload }) => {
    if (payload.imageId !== currentAsset.value?.id) return;
    metadataTaskStatus.value = payload.status;
    if (payload.metadata) imageMetadata.value = payload.metadata;
    hasMetadataError.value = payload.status === "failed";
  });
  if (!metadataTaskUpdatesActive) {
    unlisten();
    return;
  }
  metadataTaskUpdatesUnlisten = unlisten;
}

async function loadImageMetadata(imageId: string, request: number, retry = false) {
  if (!canLoadMetadata.value) return;
  try {
    if (!retry) {
      const metadata = await getImageMetadata(imageId);
      if (request !== imageMetadataRequest || currentAsset.value?.id !== imageId) return;
      imageMetadata.value = metadata;
      hasMetadataError.value = metadata?.status === "failed";
    }
    const submission = retry
      ? await retryImageMetadata(imageId)
      : await enqueueMetadataTasks([imageId]);
    if (request !== imageMetadataRequest || currentAsset.value?.id !== imageId) return;
    if (submission.acceptedImageIds.includes(imageId)) metadataTaskStatus.value = "idle";
  } catch {
    if (request === imageMetadataRequest && currentAsset.value?.id === imageId) {
      metadataTaskStatus.value = "failed";
      hasMetadataError.value = true;
    }
  }
}

function retryCurrentImageMetadata() {
  const imageId = currentAsset.value?.id;
  if (!imageId) return;
  const request = ++imageMetadataRequest;
  metadataTaskStatus.value = "idle";
  hasMetadataError.value = false;
  void loadImageMetadata(imageId, request, true);
}

async function analyzeCurrentImageColors() {
  const imageId = currentAsset.value?.id;
  if (!imageId || !canAnalyzeColors.value || isColorAnalyzing.value) return;

  isColorAnalyzing.value = true;
  hasColorAnalysisError.value = false;
  try {
    const metadata = await analyzeImageColors(imageId);
    if (currentAsset.value?.id !== imageId) return;
    colorMetadata.value = metadata;
    selectedColor.value = metadata.dominantColors[0] ?? null;
    pickedColor.value = null;
  } catch {
    if (currentAsset.value?.id === imageId) hasColorAnalysisError.value = true;
  } finally {
    if (currentAsset.value?.id === imageId) isColorAnalyzing.value = false;
  }
}

async function loadDuplicateSummary(imageId: string) {
  if (!canViewDuplicates.value) return;

  const request = ++duplicateSummaryRequest;
  isDuplicateSummaryLoading.value = true;
  hasDuplicateSummaryError.value = false;
  try {
    const summary = await listDuplicateCandidates(imageId, 0, 0);
    if (request !== duplicateSummaryRequest || currentAsset.value?.id !== imageId) return;
    duplicateSummary.value = summary;
  } catch {
    if (request === duplicateSummaryRequest && currentAsset.value?.id === imageId) {
      hasDuplicateSummaryError.value = true;
    }
  } finally {
    if (request === duplicateSummaryRequest) isDuplicateSummaryLoading.value = false;
  }
}

function openDuplicateCandidates() {
  const imageId = currentAsset.value?.id;
  if (!imageId || !canViewDuplicates.value) return;

  duplicateDialogOpen.value = true;
  duplicateCandidateImageId.value = imageId;
  duplicateCandidates.value = [];
  hasDuplicateListError.value = false;
  void loadDuplicateCandidatePage(0);
}

async function loadDuplicateCandidatePage(offset: number) {
  const imageId = currentAsset.value?.id;
  if (!imageId || !canViewDuplicates.value || isDuplicateListLoading.value) return;

  const request = ++duplicateCandidateRequest;
  isDuplicateListLoading.value = true;
  hasDuplicateListError.value = false;
  try {
    const page = await listDuplicateCandidates(imageId, offset, 24);
    if (request !== duplicateCandidateRequest || currentAsset.value?.id !== imageId) return;
    duplicateSummary.value = page;
    duplicateCandidates.value = offset === 0
      ? page.candidates
      : [...duplicateCandidates.value, ...page.candidates];
  } catch {
    if (request === duplicateCandidateRequest && currentAsset.value?.id === imageId) {
      hasDuplicateListError.value = true;
    }
  } finally {
    if (request === duplicateCandidateRequest) isDuplicateListLoading.value = false;
  }
}

function toggleColorPicker() {
  if (!canPickColor.value) return;
  colorPickerActive.value = !colorPickerActive.value;
  colorMagnifier.value = null;
  revealControls();
}

async function pickColor(event: PointerEvent) {
  const point = getOriginalImagePoint(event);
  const imageId = currentAsset.value?.id;
  if (!point || !imageId || isColorPicking.value) return;

  event.preventDefault();
  const request = ++colorPickRequest;
  isColorPicking.value = true;
  try {
    const color = await sampleImageColor(imageId, point.x, point.y);
    if (request !== colorPickRequest || currentAsset.value?.id !== imageId) return;
    selectedColor.value = color;
    pickedColor.value = { color, ...point };
    hasColorAnalysisError.value = false;
  } catch {
    if (request === colorPickRequest && currentAsset.value?.id === imageId) hasColorAnalysisError.value = true;
  } finally {
    if (request === colorPickRequest) {
      colorPickerActive.value = false;
      isColorPicking.value = false;
      colorMagnifier.value = null;
    }
  }
}

function updateColorMagnifier(event: PointerEvent) {
  const point = getOriginalImagePoint(event);
  const bounds = surface.value?.getBoundingClientRect();
  if (!point || !bounds) {
    colorMagnifier.value = null;
    return;
  }

  const diameter = COLOR_MAGNIFIER_RADIUS * 2;
  const surfaceX = event.clientX - bounds.left;
  const surfaceY = event.clientY - bounds.top;
  const left = surfaceX + COLOR_MAGNIFIER_OFFSET + diameter <= bounds.width
    ? surfaceX + COLOR_MAGNIFIER_OFFSET
    : surfaceX - COLOR_MAGNIFIER_OFFSET - diameter;
  const top = surfaceY + COLOR_MAGNIFIER_OFFSET + diameter <= bounds.height
    ? surfaceY + COLOR_MAGNIFIER_OFFSET
    : surfaceY - COLOR_MAGNIFIER_OFFSET - diameter;

  colorMagnifier.value = {
    ...point,
    left: Math.max(0, Math.min(bounds.width - diameter, left)),
    top: Math.max(0, Math.min(bounds.height - diameter, top)),
  };
}

function clearColorMagnifier() {
  colorMagnifier.value = null;
}

function closeInformationPanel() {
  informationPanelOpen.value = false;
  colorPickerActive.value = false;
  colorMagnifier.value = null;
}

function getOriginalImagePoint(event: PointerEvent) {
  const image = imageElement.value;
  const bounds = surface.value?.getBoundingClientRect();
  const fitScale = baseFitScale.value;
  if (!image?.naturalWidth || !image.naturalHeight || !bounds || fitScale === null) return null;

  const offsetX = event.clientX - bounds.left - bounds.width / 2 - viewerStore.panX;
  const offsetY = event.clientY - bounds.top - bounds.height / 2 - viewerStore.panY;
  const radians = viewerStore.rotation * Math.PI / 180;
  const unrotatedX = (Math.cos(radians) * offsetX + Math.sin(radians) * offsetY) / viewerStore.zoom;
  const unrotatedY = (-Math.sin(radians) * offsetX + Math.cos(radians) * offsetY) / viewerStore.zoom;
  const naturalX = unrotatedX / fitScale + image.naturalWidth / 2;
  const naturalY = unrotatedY / fitScale + image.naturalHeight / 2;
  if (naturalX < 0 || naturalY < 0 || naturalX >= image.naturalWidth || naturalY >= image.naturalHeight) {
    return null;
  }

  return { x: Math.floor(naturalX), y: Math.floor(naturalY) };
}

function updateFitPercentage() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight) return;

  const wasAtFit = Math.abs(viewerStore.zoom - fitZoom.value) < 0.01;
  baseFitScale.value = calculateBaseFitScale(
    surface.value.clientWidth,
    surface.value.clientHeight,
    image.naturalWidth,
    image.naturalHeight,
  );
  const nextFitZoom = fitZoom.value;
  if (wasAtFit) viewerStore.setZoom(nextFitZoom);
  else if (viewerStore.zoom < nextFitZoom) viewerStore.setZoom(nextFitZoom);
  setClampedPan(viewerStore.panX, viewerStore.panY, false);
}

function getFitZoom() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight) return 1;

  const normalFit = baseFitScale.value ?? calculateBaseFitScale(
    surface.value.clientWidth,
    surface.value.clientHeight,
    image.naturalWidth,
    image.naturalHeight,
  );
  return calculateFitZoom({
    surfaceWidth: surface.value.clientWidth,
    surfaceHeight: surface.value.clientHeight,
    imageWidth: image.naturalWidth,
    imageHeight: image.naturalHeight,
    baseFitScale: normalFit,
    rotation: viewerStore.rotation,
  });
}

function getImageMetrics() {
  const image = imageElement.value;
  if (!surface.value || !image?.naturalWidth || !image.naturalHeight || baseFitScale.value === null) return null;

  return calculateRenderedImageSize({
    imageWidth: image.naturalWidth,
    imageHeight: image.naturalHeight,
    baseFitScale: baseFitScale.value,
    zoom: viewerStore.zoom,
    rotation: viewerStore.rotation,
  });
}

function setClampedPan(panX: number, panY: number, allowOverscroll: boolean) {
  const metrics = getImageMetrics();
  if (!surface.value || !metrics) {
    viewerStore.setPan(panX, panY);
    return;
  }

  const clamped = clampImagePan({
    panX,
    panY,
    surfaceWidth: surface.value.clientWidth,
    surfaceHeight: surface.value.clientHeight,
    imageWidth: metrics.width,
    imageHeight: metrics.height,
    overscroll: allowOverscroll ? IMAGE_PAN_OVERSCROLL : 0,
  });
  viewerStore.setPan(clamped.panX, clamped.panY);
}

function zoomAt(clientX: number, clientY: number, requestedZoom: number) {
  if (!surface.value || baseFitScale.value === null) return;

  const bounds = surface.value.getBoundingClientRect();
  const next = calculateCursorCenteredZoom({
    currentZoom: viewerStore.zoom,
    requestedZoom,
    minimumZoom: fitZoom.value,
    maximumZoom: maximumZoom.value,
    panX: viewerStore.panX,
    panY: viewerStore.panY,
    clientX,
    clientY,
    surfaceLeft: bounds.left,
    surfaceTop: bounds.top,
    surfaceWidth: bounds.width,
    surfaceHeight: bounds.height,
  });
  if (!next) return;

  viewerStore.setZoom(next.zoom);
  setClampedPan(
    next.panX,
    next.panY,
    false,
  );
  showZoomHud();
}

function resetFit() {
  viewerStore.setZoom(fitZoom.value);
  viewerStore.setPan(0, 0);
  showZoomHud();
}

function setActualSize() {
  if (baseFitScale.value === null) return;
  viewerStore.setZoom(Math.min(
    maximumZoom.value,
    calculateActualSizeZoom(baseFitScale.value, fitZoom.value),
  ));
  viewerStore.setPan(0, 0);
  showZoomHud();
}

function toggleFitActualSize() {
  if (Math.abs(viewerStore.zoom - fitZoom.value) < 0.01) setActualSize();
  else resetFit();
}

function rotateClockwise() {
  const wasAtFit = Math.abs(viewerStore.zoom - fitZoom.value) < 0.01;
  const currentZoom = viewerStore.zoom;
  viewerStore.rotateClockwise();
  viewerStore.setZoom(wasAtFit ? fitZoom.value : Math.max(fitZoom.value, currentZoom));
  setClampedPan(viewerStore.panX, viewerStore.panY, false);
  showZoomHud();
}

function showZoomHud() {
  zoomHudVisible.value = true;
  if (zoomHudTimer !== null) window.clearTimeout(zoomHudTimer);
  zoomHudTimer = window.setTimeout(() => {
    zoomHudVisible.value = false;
    zoomHudTimer = null;
  }, 900);
}

function revealControls() {
  controlsVisible.value = true;
  if (controlsTimer !== null) window.clearTimeout(controlsTimer);
  controlsTimer = window.setTimeout(() => {
    controlsVisible.value = false;
    backgroundMenuOpen.value = false;
    controlsTimer = null;
  }, 1800);
}

function navigate(direction: -1 | 1) {
  const targetImageId = direction < 0 ? previousImageId.value : nextImageId.value;
  if (!targetImageId) return;

  viewerStore.selectImage(targetImageId);
  revealControls();
}

function handlePointerDown(event: PointerEvent) {
  if (colorPickerActive.value) {
    if (event.button === 0) pickColor(event);
    return;
  }

  const canPanWithLeftButton = event.button === 0 && viewerStore.zoom > fitZoom.value + 0.01;
  if (event.button !== 1 && !canPanWithLeftButton) return;
  if (!surface.value) return;

  event.preventDefault();
  dragState = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    panX: viewerStore.panX,
    panY: viewerStore.panY,
  };
  isDragging.value = true;
  surface.value.setPointerCapture(event.pointerId);
}

function handlePointerMove(event: PointerEvent) {
  if (colorPickerActive.value) {
    if (!isColorPicking.value) updateColorMagnifier(event);
    return;
  }

  if (!dragState || event.pointerId !== dragState.pointerId) return;

  setClampedPan(
    dragState.panX + event.clientX - dragState.startX,
    dragState.panY + event.clientY - dragState.startY,
    true,
  );
}

function handlePointerEnd(event: PointerEvent) {
  if (!dragState || event.pointerId !== dragState.pointerId) return;

  if (surface.value?.hasPointerCapture(event.pointerId)) surface.value.releasePointerCapture(event.pointerId);
  dragState = null;
  isDragging.value = false;
  setClampedPan(viewerStore.panX, viewerStore.panY, false);
}

function returnToGallery() {
  const context = viewerStore.complete();
  void router.push(context?.returnPath ?? "/library");
}

function setBackground(background: ViewerBackground) {
  viewerStore.setBackground(background);
  backgroundMenuOpen.value = false;
  revealControls();
}

function openCurrentReference() {
  if (!currentAsset.value) return;
  void openReferenceWindow(currentAsset.value).catch(() => undefined);
}

async function toggleFullscreen() {
  try {
    if (isTauriEnvironment) {
      const appWindow = getCurrentWindow();
      const nextFullscreen = !(await appWindow.isFullscreen());
      await appWindow.setFullscreen(nextFullscreen);
      return;
    }

    if (document.fullscreenElement) await document.exitFullscreen();
    else await surface.value?.requestFullscreen();
  } catch {
    // Keep the viewer usable when the host does not allow fullscreen.
  }
}

async function exitFullscreen() {
  try {
    if (isTauriEnvironment) {
      const appWindow = getCurrentWindow();
      if (!(await appWindow.isFullscreen())) return false;
      await appWindow.setFullscreen(false);
      return true;
    }

    if (!document.fullscreenElement) return false;
    await document.exitFullscreen();
    return true;
  } catch {
    return false;
  }
}

function handleWheel(event: WheelEvent) {
  if (event.deltaY === 0) return;
  event.preventDefault();

  if (event.altKey) {
    const now = performance.now();
    if (now - wheelTimestamp < 120) return;
    wheelTimestamp = now;
    navigate(event.deltaY > 0 ? 1 : -1);
    return;
  }

  const delta = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * 16 : event.deltaY;
  zoomAt(event.clientX, event.clientY, viewerStore.zoom * Math.exp(-delta * 0.0015));
  revealControls();
}

function openTagManager() {
  tagDialogOpen.value = true;
}

async function removeCurrentTag(tagId: string) {
  const imageId = currentAsset.value?.id;
  if (!imageId) return;
  if (!await tagStore.removeFromImages(tagId, [imageId])) return;
  imageTags.value = imageTags.value.filter((tag) => tag.id !== tagId);
  tagStore.imageTagsByImageId = { ...tagStore.imageTagsByImageId, [imageId]: imageTags.value };
}

async function refreshCurrentTags() {
  const imageId = currentAsset.value?.id;
  if (!imageId) return;
  imageTags.value = await tagStore.loadImageTags(imageId);
}
</script>

<template>
  <section
    class="viewer-page"
    :class="backgroundClass"
    :aria-label="currentAsset?.filename ?? $t('viewer.title')"
    @mousemove="revealControls"
  >
    <div
      v-if="hasSession"
      ref="surface"
      class="viewer-page__surface"
      :class="{
        'is-dragging': isDragging,
        'is-pan-enabled': viewerStore.zoom > fitZoom + 0.01,
        'is-color-picking': colorPickerActive,
      }"
      @wheel="handleWheel"
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerleave="clearColorMagnifier"
      @pointerup="handlePointerEnd"
      @pointercancel="handlePointerEnd"
      @dblclick="!colorPickerActive && toggleFitActualSize()"
    >
      <img
        v-if="thumbnailPreviewSource && !isFullImageLoaded && !previewFailed && !showError"
        class="viewer-page__preview-image"
        :src="thumbnailPreviewSource"
        alt=""
        aria-hidden="true"
        decoding="async"
        @error="previewFailed = true"
      >
      <Transition name="viewer-image">
        <img
          v-if="imageSource && !showError"
          :key="currentAsset?.id"
          ref="imageElement"
          class="viewer-page__image"
          :style="imageStyle"
          :src="imageSource"
          :alt="currentAsset?.filename ?? ''"
          decoding="async"
          @load="handleFullImageLoad"
          @error="imageFailed = true"
        >
      </Transition>
      <div
        v-if="colorPickerActive && colorMagnifier"
        class="viewer-page__color-magnifier"
        :style="colorMagnifierStyle"
        aria-hidden="true"
      >
        <span class="viewer-page__color-magnifier-grid" />
        <span class="viewer-page__color-magnifier-center" />
      </div>
      <p v-if="(viewerStore.isLoading || isResolvingAsset) && !showError" class="viewer-page__state">
        {{ $t("viewer.loading") }}
      </p>
      <div v-else-if="showError" class="viewer-page__state viewer-page__state--error" role="status">
        <p>{{ errorMessage }}</p>
        <p v-if="currentAsset?.path" class="viewer-page__path" :title="currentAsset.path">
          {{ currentAsset.path }}
        </p>
      </div>
    </div>

    <div v-else class="viewer-page__state viewer-page__state--error">
      <p>{{ $t("viewer.sessionUnavailable") }}</p>
      <button type="button" class="viewer-page__return" @click="returnToGallery">
        {{ $t("viewer.back") }}
      </button>
    </div>

    <header v-show="controlsVisible && hasSession" class="viewer-page__topbar">
      <div class="viewer-page__title-group iv-glass-surface">
        <AppTooltip :content="$t('viewer.back')" side="bottom">
          <template #trigger>
            <button type="button" :aria-label="$t('viewer.back')" @click="returnToGallery">
              <ArrowLeft :size="18" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <p :title="currentAsset?.filename">{{ currentAsset?.filename }}</p>
      </div>

      <div class="viewer-page__actions iv-glass-surface">
        <AppTooltip :content="$t('viewer.manageTags')" side="bottom">
          <template #trigger>
            <button
              type="button"
              :aria-label="$t('viewer.manageTags')"
              :disabled="!currentAsset"
              @click="tagDialogOpen = true"
            >
              <Tags :size="18" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <AppTooltip :content="$t('viewer.addToCollection')" side="bottom">
          <template #trigger>
            <button
              type="button"
              :aria-label="$t('viewer.addToCollection')"
              :disabled="!currentAsset"
              @click="collectionDialogOpen = true"
            >
              <FolderPlus :size="18" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <AppTooltip :content="$t('viewer.favorite')" side="bottom">
          <template #trigger>
            <span class="viewer-page__disabled-action" tabindex="0">
              <button type="button" :aria-label="$t('viewer.favorite')" disabled>
                <Heart :size="18" :stroke-width="1.8" aria-hidden="true" />
              </button>
            </span>
          </template>
        </AppTooltip>
        <AppTooltip :content="$t('viewer.reference')" side="bottom">
          <template #trigger>
            <button
              type="button"
              :aria-label="$t('viewer.reference')"
              :disabled="!currentAsset"
              @click="openCurrentReference"
            >
              <Bookmark :size="18" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <AppTooltip :content="$t('viewer.details')" side="bottom">
          <template #trigger>
            <button
              type="button"
              :class="{ 'is-active': informationPanelOpen }"
              :aria-label="$t('viewer.details')"
              :aria-pressed="informationPanelOpen"
              :disabled="!currentAsset"
              @click="informationPanelOpen = !informationPanelOpen"
            >
              <Info :size="18" :stroke-width="1.8" aria-hidden="true" />
            </button>
          </template>
        </AppTooltip>
        <div class="viewer-page__more-wrap">
          <AppTooltip :content="$t('viewer.more')" side="bottom">
            <template #trigger>
              <button
                type="button"
                :aria-label="$t('viewer.more')"
                :aria-expanded="backgroundMenuOpen"
                @click="backgroundMenuOpen = !backgroundMenuOpen"
              >
                <Ellipsis :size="18" :stroke-width="1.8" aria-hidden="true" />
              </button>
            </template>
          </AppTooltip>
          <div v-if="backgroundMenuOpen" class="viewer-page__background-menu iv-glass-surface" role="menu">
            <p>{{ $t("viewer.background") }}</p>
            <button
              v-for="background in backgrounds"
              :key="background"
              type="button"
              role="menuitemradio"
              :aria-checked="viewerStore.background === background"
              :class="{ 'is-active': viewerStore.background === background }"
              @click="setBackground(background)"
            >
              {{ $t(`viewer.background${background[0].toUpperCase()}${background.slice(1)}`) }}
            </button>
            <button class="viewer-page__menu-action" type="button" role="menuitem" @click="rotateClockwise">
              <RotateCw :size="16" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ $t("viewer.rotateClockwise") }}</span>
            </button>
          </div>
        </div>
      </div>
    </header>

    <nav v-show="controlsVisible && hasSession" class="viewer-page__navigation" :aria-label="$t('viewer.navigation')">
      <AppTooltip :content="$t('viewer.previous')" side="right">
        <template #trigger>
          <button class="iv-glass-surface" type="button" :aria-label="$t('viewer.previous')" :disabled="!previousImageId" @click="navigate(-1)">
            <ChevronLeft :size="22" :stroke-width="1.8" aria-hidden="true" />
          </button>
        </template>
      </AppTooltip>
      <AppTooltip :content="$t('viewer.next')" side="left">
        <template #trigger>
          <button class="iv-glass-surface" type="button" :aria-label="$t('viewer.next')" :disabled="!nextImageId" @click="navigate(1)">
            <ChevronRight :size="22" :stroke-width="1.8" aria-hidden="true" />
          </button>
        </template>
      </AppTooltip>
    </nav>

    <footer v-show="controlsVisible && hasSession" class="viewer-page__status iv-glass-surface">
      <span class="viewer-page__counter">
        {{ $t('viewer.position', { current: currentIndex + 1, total: viewerStore.resultImageIds.length }) }}
      </span>
    </footer>

    <div v-show="zoomHudVisible && hasSession" class="viewer-page__zoom-hud iv-glass-surface" aria-live="polite">
      {{ zoomLabel }}
    </div>

    <ImageInformationPanel
      :open="informationPanelOpen && hasSession"
      :filename="currentAsset?.filename ?? null"
      :image-metadata="imageMetadata"
      :metadata-task-status="metadataTaskStatus"
      :has-metadata-error="hasMetadataError"
      :can-load-metadata="canLoadMetadata"
      :metadata="colorMetadata"
      :selected-color="selectedColor"
      :picked-color="pickedColor"
      :is-analyzing="isColorAnalyzing"
      :has-error="hasColorAnalysisError"
      :picker-active="colorPickerActive"
      :can-analyze="canAnalyzeColors"
      :can-pick="canPickColor"
      :duplicate-summary="duplicateSummary"
      :is-duplicates-loading="isDuplicateSummaryLoading"
      :has-duplicates-error="hasDuplicateSummaryError"
      :can-view-duplicates="canViewDuplicates"
      :tags="imageTags"
      @close="closeInformationPanel"
      @retry-metadata="retryCurrentImageMetadata"
      @analyze="analyzeCurrentImageColors"
      @toggle-picker="toggleColorPicker"
      @select-color="selectedColor = $event; pickedColor = null"
      @open-duplicates="openDuplicateCandidates"
      @manage-tags="openTagManager"
      @remove-tag="removeCurrentTag"
    />

    <DuplicateCandidateDialog
      :open="duplicateDialogOpen && duplicateCandidateImageId === currentAsset?.id"
      :source="currentAsset"
      :candidates="duplicateCandidates"
      :total-count="duplicateSummary?.totalCount ?? 0"
      :is-loading="isDuplicateListLoading"
      :has-error="hasDuplicateListError"
      :records-by-image-id="thumbnailStore.recordsByImageId"
      :generating-label="$t('home.thumbnailGenerating')"
      :unavailable-label="$t('home.thumbnailUnavailable')"
      @update:open="duplicateDialogOpen = $event"
      @load-more="loadDuplicateCandidatePage(duplicateCandidates.length)"
    />

    <AddToCollectionDialog
      :open="collectionDialogOpen"
      :image-ids="currentAsset ? [currentAsset.id] : []"
      @update:open="collectionDialogOpen = $event"
    />
    <TagAssignmentDialog
      :open="tagDialogOpen"
      :image-ids="currentAsset ? [currentAsset.id] : []"
      @update:open="tagDialogOpen = $event"
      @updated="refreshCurrentTags"
    />
  </section>
</template>

<style scoped lang="scss">
.viewer-page {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  color: var(--iv-overlay-text-primary);
  --iv-viewer-matte-active-surface: var(--iv-viewer-matte-auto-surface);
  background-color: var(--iv-viewer-matte-active-surface);
}

.viewer-page--background-dark {
  --iv-viewer-matte-active-surface: var(--iv-viewer-matte-surface);
}

.viewer-page--background-light {
  --iv-viewer-matte-active-surface: var(--iv-viewer-matte-surface-light);
}

.viewer-page__surface {
  position: relative;
  display: grid;
  place-items: center;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  padding: var(--iv-space-6);
  touch-action: none;
  background-color: var(--iv-viewer-matte-active-surface);
}

.viewer-page__image {
  position: relative;
  z-index: 1;
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  flex: none;
  transform-origin: center;
  will-change: transform;
  transition: transform var(--iv-motion-fast) ease-out;
}

.viewer-page__preview-image {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  pointer-events: none;
}

.viewer-page__surface.is-pan-enabled .viewer-page__image {
  cursor: grab;
}

.viewer-page__surface.is-color-picking,
.viewer-page__surface.is-color-picking .viewer-page__image {
  cursor: crosshair;
}

.viewer-page__color-magnifier {
  position: absolute;
  z-index: 1;
  box-sizing: border-box;
  overflow: hidden;
  background-color: var(--iv-viewer-matte-active-surface);
  background-repeat: no-repeat;
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-round);
  box-shadow: 0 8px 20px rgb(0 0 0 / 40%);
  image-rendering: pixelated;
  pointer-events: none;
}

.viewer-page__color-magnifier-grid,
.viewer-page__color-magnifier-center {
  position: absolute;
  inset: 0;
}

.viewer-page__color-magnifier-grid {
  background-image:
    linear-gradient(to right, rgb(255 255 255 / 60%) 0 1px, transparent 1px),
    linear-gradient(to bottom, rgb(255 255 255 / 60%) 0 1px, transparent 1px);
  background-position: var(--iv-color-magnifier-grid-offset) var(--iv-color-magnifier-grid-offset);
  background-size: var(--iv-color-magnifier-pixel-size) var(--iv-color-magnifier-pixel-size);
}

.viewer-page__color-magnifier-center {
  inset: auto;
  top: 50%;
  left: 50%;
  box-sizing: border-box;
  width: var(--iv-color-magnifier-pixel-size);
  height: var(--iv-color-magnifier-pixel-size);
  border: 2px solid #FFF;
  box-shadow: 0 0 0 1px rgb(0 0 0 / 75%);
  transform: translate(-50%, -50%);
}

.viewer-page__surface.is-dragging .viewer-page__image {
  cursor: grabbing;
  transition: none;
}

.viewer-page__state {
  margin: 0;
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.viewer-page__state--error {
  display: grid;
  place-content: center;
  justify-items: center;
  gap: var(--iv-space-3);
  width: 100%;
  height: 100%;
  padding: var(--iv-space-6);
  color: var(--iv-overlay-text-secondary);
  text-align: center;
}

.viewer-page__path {
  max-width: min(100%, 720px);
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--iv-overlay-text-tertiary);
  font-size: var(--iv-font-size-12);
}

.viewer-page__return {
  min-height: var(--iv-control-height);
  padding: 0 var(--iv-space-3);
  color: var(--iv-overlay-text-primary);
  font: inherit;
  background: var(--iv-overlay-surface);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
}

.viewer-page__topbar,
.viewer-page__navigation,
.viewer-page__status {
  position: absolute;
  z-index: 1;
}

.viewer-page__topbar {
  inset: var(--iv-space-4) var(--iv-space-5) auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--iv-space-4);
  pointer-events: none;
}

.viewer-page__title-group,
.viewer-page__actions {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: var(--iv-space-2);
  pointer-events: auto;
  padding: var(--iv-control-inset);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-md);
}

.viewer-page__actions > * {
  flex: 0 0 auto;
}

.viewer-page__title-group p {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  font-size: var(--iv-font-size-14);
  font-weight: var(--iv-font-weight-medium);
  line-height: var(--iv-line-height-20);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.viewer-page button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--iv-control-height);
  height: var(--iv-control-height);
  padding: 0;
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-overlay-surface);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  cursor: pointer;
  transition: background-color var(--iv-motion-fast) ease, opacity var(--iv-motion-fast) ease;
}

.viewer-page__title-group > button,
.viewer-page__actions > button,
.viewer-page__actions > .viewer-page__disabled-action > button {
  background-color: transparent;
  border-color: transparent;
}

.viewer-page button:hover:not(:disabled) {
  background-color: var(--iv-overlay-surface-hover);
}

.viewer-page button.is-active {
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-overlay-accent-active);
  border-color: var(--iv-accent);
}

.viewer-page button:focus-visible,
.viewer-page__disabled-action:focus-visible {
  outline: var(--iv-focus-ring-width) solid var(--iv-accent);
  outline-offset: var(--iv-space-1);
}

.viewer-page button:disabled {
  cursor: default;
  opacity: 0.42;
}

.viewer-page__disabled-action {
  display: inline-flex;
  border-radius: var(--iv-radius-sm);
}

.viewer-page__more-wrap {
  position: relative;
}

.viewer-page__background-menu {
  position: absolute;
  top: calc(100% + var(--iv-space-2));
  right: 0;
  display: grid;
  gap: var(--iv-space-1);
  width: 144px;
  padding: var(--iv-space-2);
  color: var(--iv-overlay-text-primary);
  background-color: var(--iv-glass-bg-strong);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  box-shadow: var(--iv-shadow-floating);
}

.viewer-page__background-menu p {
  margin: var(--iv-space-1) var(--iv-space-2);
  color: var(--iv-overlay-text-secondary);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
}

.viewer-page__background-menu button {
  justify-content: flex-start;
  width: 100%;
  height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  font: inherit;
  font-size: var(--iv-font-size-13);
}

.viewer-page__background-menu button.is-active {
  background-color: var(--iv-overlay-accent-active);
}

.viewer-page__background-menu .viewer-page__menu-action {
  gap: var(--iv-space-2);
}

.viewer-page__navigation {
  inset: 50% var(--iv-space-5) auto;
  display: flex;
  justify-content: space-between;
  width: calc(100% - var(--iv-space-10));
  pointer-events: none;
  transform: translateY(-50%);
}

.viewer-page__navigation :deep(button) {
  pointer-events: auto;
}

.viewer-page__status {
  inset: auto auto var(--iv-space-4) 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 72px;
  height: var(--iv-control-small-height);
  padding: 0 var(--iv-space-2);
  color: var(--iv-overlay-text-status);
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  background-color: var(--iv-overlay-surface);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-round);
  transform: translateX(-50%);
}

.viewer-page__counter {
  overflow: hidden;
  font-variant-numeric: tabular-nums;
  text-overflow: clip;
  white-space: nowrap;
}

.viewer-page__zoom-hud {
  position: absolute;
  inset: 50% auto auto 50%;
  z-index: 1;
  padding: var(--iv-space-2) var(--iv-space-3);
  color: var(--iv-overlay-text-primary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  background-color: var(--iv-overlay-surface-hud);
  border: 1px solid var(--iv-overlay-border);
  border-radius: var(--iv-radius-sm);
  pointer-events: none;
  transform: translate(-50%, -50%);
}

.viewer-image-enter-active,
.viewer-image-leave-active {
  transition: opacity var(--iv-motion-fast) ease;
}

.viewer-image-enter-from,
.viewer-image-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .viewer-image-enter-active,
  .viewer-image-leave-active,
  .viewer-page button {
    transition: none;
  }
}
</style>
