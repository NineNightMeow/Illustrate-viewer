import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test, { after } from "node:test";
import { createServer } from "vite";

const vite = await createServer({
  appType: "custom",
  server: { middlewareMode: true },
});
const { createVirtualGridLayout, createVirtualGridRange } = await vite.ssrLoadModule(
  "/src/features/gallery/virtualGrid.ts",
);

after(() => vite.close());

test("a 10k search result schedules only the viewport and overscan window", () => {
  const resultCount = 10_000;
  const layout = createVirtualGridLayout(1280, resultCount);
  const viewport = createVirtualGridRange(layout, 0, 720, 0);
  const overscan = createVirtualGridRange(layout, 0, 720);
  const requested = overscan.endIndex - overscan.startIndex;

  assert.ok(viewport.endIndex > 0);
  assert.ok(requested > viewport.endIndex - viewport.startIndex);
  assert.ok(requested < resultCount);
  assert.ok(requested <= layout.columns * 7);
});

test("1k and 10k virtual grids keep DOM work bounded by the viewport window", () => {
  for (const resultCount of [1_000, 10_000]) {
    const layout = createVirtualGridLayout(1280, resultCount);
    const range = createVirtualGridRange(layout, layout.rowHeight * 37, 720);

    assert.equal(layout.itemCount, resultCount);
    assert.ok(range.endIndex - range.startIndex <= layout.columns * 8);
    assert.ok(range.endIndex < resultCount);
    assert.ok(range.startIndex >= 0);
  }
});

test("a viewport change keeps thumbnail requests inside the new visible window and overscan", () => {
  const layout = createVirtualGridLayout(1280, 10_000);
  const firstWindow = createVirtualGridRange(layout, 0, 720);
  const nextViewport = createVirtualGridRange(layout, layout.rowHeight * 20, 720, 0);
  const nextWindow = createVirtualGridRange(layout, layout.rowHeight * 20, 720);

  assert.ok(nextWindow.startIndex > firstWindow.endIndex);
  assert.ok(nextWindow.startIndex <= nextViewport.startIndex);
  assert.ok(nextWindow.endIndex >= nextViewport.endIndex);
  assert.ok(nextWindow.endIndex - nextWindow.startIndex <= (nextViewport.endIndex - nextViewport.startIndex) + layout.columns * 4);
  assert.ok(nextWindow.endIndex < layout.itemCount);
});

test("GalleryGrid renders the bounded range instead of the complete result array", async () => {
  const source = await readFile(new URL("../src/features/gallery/components/GalleryGrid.vue", import.meta.url), "utf8");

  assert.match(source, /const visibleAssets = computed\(\(\) =>[\s\S]*props\.assets\.slice\(range\.value\.startIndex, range\.value\.endIndex\)/);
  assert.match(source, /v-for="asset in visibleAssets"/);
  assert.match(source, /:style="\{ height: `\$\{layout\.totalHeight\}px` \}"/);
  assert.doesNotMatch(source, /v-for="asset in props\.assets"/);
});

test("SearchPage delegates thumbnail work to its GalleryGrid window only", async () => {
  const source = await readFile(new URL("../src/features/search/SearchPage.vue", import.meta.url), "utf8");

  assert.doesNotMatch(source, /ensureThumbnails\s*\(/);
  assert.match(source, /syncThumbnailWindow\(thumbnailWindowScope, window\.viewport, window\.prefetch\)/);
  assert.match(source, /syncThumbnailWindow\(thumbnailWindowScope, \[\], \[\]\)/);
});

test("large result pages use stable thumbnail scopes and release them on replacement/unmount", async () => {
  const paths = [
    "src/features/favorites/FavoritesPage.vue",
    "src/features/collections/CollectionDetailPage.vue",
    "src/features/smartCollections/SmartCollectionDetailPage.vue",
    "src/features/tags/TagDetailPage.vue",
  ];
  const sources = await Promise.all(paths.map((path) => readFile(new URL(`../${path}`, import.meta.url), "utf8")));

  for (const source of sources) {
    assert.doesNotMatch(source, /ensureThumbnails\s*\(/);
    assert.match(source, /syncThumbnailWindow\(/);
    assert.match(source, /releaseThumbnailWindow\(/);
    assert.match(source, /@thumbnail-window-change="requestThumbnailWindow"/);
  }
});

test("thumbnail window synchronization coalesces rapid viewport replacement", async () => {
  const source = await readFile(new URL("../src/features/library/thumbnailStore.ts", import.meta.url), "utf8");

  assert.match(source, /pendingThumbnailWindow: ThumbnailWindow \| null/);
  assert.match(source, /while \(pendingThumbnailWindow\)/);
  assert.match(source, /pendingThumbnailWindow = window/);
  assert.match(source, /releaseThumbnailWindow[\s\S]*pendingThumbnailWindow = null/);
  assert.match(source, /const viewportIds = new Set/);
  assert.match(source, /!viewportIds\.has\(asset\.id\)/);
});

test("duplicate indicators and candidate dialogs stay bounded to visible batches/pages", async () => {
  const [gallery, viewer, dialog] = await Promise.all([
    readFile(new URL("../src/features/gallery/GalleryPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/viewer/ViewerPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/fingerprint/DuplicateCandidateDialog.vue", import.meta.url), "utf8"),
  ]);

  assert.match(gallery, /for \(let start = 0; start < imageIds\.length; start \+= 128\)/);
  assert.match(gallery, /const imageIds = \[\.\.\.new Set\(windowAssets\.map/);
  assert.match(viewer, /listDuplicateCandidates\(imageId, offset, 24\)/);
  assert.match(viewer, /ensureThumbnails\(\[source, \.\.\.candidates\.map/);
  assert.match(dialog, /v-for="candidate in candidates"/);
  assert.match(dialog, /max-height: min\(54vh, 520px\)/);
});

test("gallery/search/filter paths do not bulk-enqueue metadata, fingerprint, or color analysis", async () => {
  const paths = [
    "src/features/gallery/GalleryPage.vue",
    "src/features/search/SearchPage.vue",
    "src/features/favorites/FavoritesPage.vue",
    "src/features/collections/CollectionDetailPage.vue",
    "src/features/smartCollections/SmartCollectionDetailPage.vue",
    "src/features/tags/TagDetailPage.vue",
  ];
  const sources = await Promise.all(paths.map((path) => readFile(new URL(`../${path}`, import.meta.url), "utf8")));
  const bulkAnalysis = /enqueueMetadataTasks|analyzeImageColors|analyzeImageFingerprint|listDuplicateCandidates/;

  for (const source of sources) assert.doesNotMatch(source, bulkAnalysis);
});
