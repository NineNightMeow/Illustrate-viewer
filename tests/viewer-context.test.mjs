import assert from "node:assert/strict";
import test, { after } from "node:test";
import { createPinia, setActivePinia } from "pinia";
import { createServer } from "vite";

const vite = await createServer({
  appType: "custom",
  server: { middlewareMode: true },
});
const { imageLibraryIdsFor, useViewerStore } = await vite.ssrLoadModule("/src/features/viewer/viewerStore.ts");
const { useSearchStore } = await vite.ssrLoadModule("/src/features/search/searchStore.ts");

after(() => vite.close());

const searchAssets = [
  { id: "library-a/image-1", libraryId: "library-a" },
  { id: "library-b/image-2", libraryId: "library-b" },
  { id: "library-a/image-3", libraryId: "library-a" },
];

function createViewer() {
  setActivePinia(createPinia());
  return useViewerStore();
}

function searchFilters() {
  return {
    libraryId: null,
    collectionId: null,
    tagIds: ["tag-flowers"],
    favoritesOnly: false,
    formats: ["png"],
    metadata: { minimumWidth: 1920 },
    smartCollectionId: null,
  };
}

test("cross-library search keeps snapshot order and restores its search context", () => {
  const viewer = createViewer();
  const searchStore = useSearchStore();
  searchStore.query = "flower";
  searchStore.filters = searchFilters();

  viewer.open({
    source: "search",
    entryLibraryId: "library-a",
    resultImageIds: searchAssets.map((asset) => asset.id),
    imageLibraryIds: imageLibraryIdsFor(searchAssets),
    activeImageId: "library-a/image-1",
    returnPath: "/search",
    sourceId: null,
    selectedImageIds: ["library-a/image-1"],
    scrollTop: 288,
    layoutMode: "grid",
    thumbnailSize: null,
    search: searchStore.snapshotViewContext(),
  });
  searchStore.filters.tagIds.push("tag-mutated-after-open");
  searchStore.filters.metadata.minimumWidth = 1;

  assert.deepEqual(viewer.resultImageIds, searchAssets.map((asset) => asset.id));
  assert.equal(viewer.libraryIdFor("library-b/image-2"), "library-b");

  viewer.selectImage("library-b/image-2");
  assert.equal(viewer.currentIndex, 1);
  assert.equal(viewer.libraryIdFor(viewer.currentImageId), "library-b");

  const completed = viewer.complete();
  assert.equal(completed.returnPath, "/search");
  assert.equal(completed.activeImageId, "library-b/image-2");
  assert.deepEqual(completed.search, {
    query: "flower",
    filters: searchFilters(),
  });

  const returned = viewer.consumeReturnContext("search");
  searchStore.clear();
  searchStore.restoreViewContext(returned.search);
  assert.equal(searchStore.query, "flower");
  assert.deepEqual(searchStore.filters, searchFilters());
});

test("library, favorites, collection, and smart collection retain their own return contexts", () => {
  const sources = [
    { source: "library", sourceId: "library-a", returnPath: "/library/library-a" },
    { source: "favorites", sourceId: null, returnPath: "/favorites" },
    { source: "collection", sourceId: "collection-1", returnPath: "/collections/collection-1" },
    { source: "smart-collection", sourceId: "smart-1", returnPath: "/smart-collections/smart-1" },
  ];

  for (const context of sources) {
    const viewer = createViewer();
    viewer.open({
      ...context,
      entryLibraryId: "library-a",
      resultImageIds: ["library-a/image-1", "library-a/image-3"],
      imageLibraryIds: {
        "library-a/image-1": "library-a",
        "library-a/image-3": "library-a",
      },
      activeImageId: "library-a/image-1",
      selectedImageIds: ["library-a/image-1"],
      scrollTop: 144,
      layoutMode: "grid",
      thumbnailSize: null,
      sort: "name",
      fileType: "all",
    });

    viewer.selectImage("library-a/image-3");
    const returned = viewer.complete();
    assert.equal(returned.source, context.source);
    assert.equal(returned.sourceId, context.sourceId);
    assert.equal(returned.returnPath, context.returnPath);
    assert.equal(returned.activeImageId, "library-a/image-3");
    assert.equal(returned.scrollTop, 144);
    assert.deepEqual(viewer.consumeReturnContext(context.source, context.sourceId), returned);
  }
});

test("a stale image mapping does not discard the snapshot navigation sequence", () => {
  const viewer = createViewer();
  viewer.open({
    source: "search",
    entryLibraryId: "library-a",
    resultImageIds: searchAssets.map((asset) => asset.id),
    imageLibraryIds: imageLibraryIdsFor(searchAssets),
    activeImageId: "library-b/image-2",
    returnPath: "/search",
    sourceId: null,
    selectedImageIds: ["library-b/image-2"],
    scrollTop: 0,
    search: { query: "flower", filters: searchFilters() },
  });
  delete viewer.context.imageLibraryIds["library-b/image-2"];

  assert.equal(viewer.libraryIdFor(viewer.currentImageId), null);
  assert.equal(viewer.currentIndex, 1);
  viewer.selectImage("library-a/image-3");
  assert.equal(viewer.currentIndex, 2);
  assert.equal(viewer.currentImageId, "library-a/image-3");
});

test("stale search responses cannot overwrite a newer search or clear state", async () => {
  const pending = [];
  const originalWindow = globalThis.window;
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: (_command, { filters }) => new Promise((resolve) => {
        pending.push({ query: filters.query, resolve });
      }),
    },
  };

  const isolatedVite = await createServer({ appType: "custom", server: { middlewareMode: true, hmr: false } });
  try {
    const { useSearchStore: useIsolatedSearchStore } = await isolatedVite.ssrLoadModule(
      "/src/features/search/searchStore.ts",
    );
    setActivePinia(createPinia());
    const store = useIsolatedSearchStore();

    store.query = "A";
    const searchA = store.search();
    store.query = "B";
    const searchB = store.search();
    pending.find((request) => request.query === "B").resolve([searchAssets[1]]);
    await searchB;
    pending.find((request) => request.query === "A").resolve([searchAssets[0]]);
    await searchA;

    assert.deepEqual(store.assets, [searchAssets[1]]);
    assert.equal(store.isLoading, false);

    store.query = "C";
    const searchC = store.search();
    store.clear();
    pending.find((request) => request.query === "C").resolve([searchAssets[2]]);
    await searchC;

    assert.deepEqual(store.assets, []);
    assert.equal(store.hasSearched, false);
    assert.equal(store.isLoading, false);
  } finally {
    await isolatedVite.close();
    if (originalWindow === undefined) delete globalThis.window;
    else globalThis.window = originalWindow;
  }
});
