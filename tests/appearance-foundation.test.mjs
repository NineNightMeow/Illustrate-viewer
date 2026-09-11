import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test, { after } from "node:test";
import { createPinia, setActivePinia } from "pinia";
import { createServer } from "vite";

const vite = await createServer({
  appType: "custom",
  server: { middlewareMode: true },
});
const { APP_SETTINGS_STORAGE_KEY, useAppStore } = await vite.ssrLoadModule("/src/app/appStore.ts");
const { useTheme } = await vite.ssrLoadModule("/src/shared/composables/useTheme.ts");

after(() => vite.close());

function createStorage(initialValue) {
  let value = initialValue;
  return {
    getItem: (key) => key === APP_SETTINGS_STORAGE_KEY ? value : null,
    setItem: (key, nextValue) => {
      if (key === APP_SETTINGS_STORAGE_KEY) value = nextValue;
    },
    saved: () => value,
  };
}

function hydrateSettings(saved) {
  const storage = createStorage(JSON.stringify(saved));
  globalThis.window = { localStorage: storage };
  setActivePinia(createPinia());
  const store = useAppStore();
  store.hydrate();
  return { store, storage };
}

test("legacy appearance settings migrate with safe M8 defaults", () => {
  const { store, storage } = hydrateSettings({
    theme: "dark",
    accentColor: "#D9823A",
    liquidGlassEnabled: true,
  });

  assert.equal(store.appearance.theme, "dark");
  assert.equal(store.appearance.accentColor, "#D9823A");
  assert.equal(store.appearance.liquidGlass.enabled, true);
  assert.deepEqual(store.appearance.customBackground, {
    enabled: false,
    source: null,
    fit: "cover",
    opacity: 100,
    blur: 0,
    dim: 0,
  });

  store.persist();
  const persisted = JSON.parse(storage.saved());
  assert.deepEqual(persisted.appearance, store.appearance);
  assert.equal("theme" in persisted, false);
  assert.equal("liquidGlassEnabled" in persisted, false);
});

test("partial or malformed appearance settings retain defaults", () => {
  const { store } = hydrateSettings({
    appearance: {
      theme: "light",
      accentColor: "not-a-color",
      customBackground: { enabled: true, opacity: 999, fit: "stretch" },
      liquidGlass: { intensity: -1 },
    },
  });

  assert.equal(store.appearance.theme, "light");
  assert.equal(store.appearance.accentColor, "#6B87F2");
  assert.equal(store.appearance.customBackground.enabled, true);
  assert.equal(store.appearance.customBackground.opacity, 100);
  assert.equal(store.appearance.customBackground.fit, "cover");
  assert.equal(store.appearance.liquidGlass.enabled, false);
  assert.equal(store.appearance.liquidGlass.intensity, 50);
});

test("appearance state updates root attributes without visual effects", () => {
  const properties = new Map();
  globalThis.document = {
    documentElement: {
      dataset: {},
      style: { setProperty: (name, value) => properties.set(name, value) },
    },
  };
  const { store } = hydrateSettings({
    appearance: {
      theme: "dark",
      accentColor: "#8B68D9",
      customBackground: { enabled: true },
      liquidGlass: { enabled: true },
    },
  });

  useTheme().applyAppearance(store.appearance);

  assert.deepEqual(globalThis.document.documentElement.dataset, {
    theme: "dark",
    accent: "#8B68D9",
    backgroundEnabled: "true",
    liquidGlass: "true",
  });
  assert.equal(properties.get("--iv-accent"), "#8B68D9");
});

test("image-critical surfaces do not consume application appearance tokens", async () => {
  const [viewer, preview, reference, thumbnail, referenceEntry] = await Promise.all([
    readFile(new URL("../src/features/viewer/ViewerPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/gallery/components/QuickPreviewOverlay.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/reference/ReferenceWindowPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/library/components/ThumbnailPreview.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/reference-main.ts", import.meta.url), "utf8"),
  ]);

  assert.match(viewer, /--iv-viewer-matte-active-surface/);
  assert.match(preview, /--iv-viewer-matte-auto-surface/);
  assert.match(thumbnail, /--iv-thumbnail-matte-surface/);
  assert.match(reference, /--iv-image-critical-surface/);
  for (const source of [viewer, preview, reference, thumbnail]) {
    assert.doesNotMatch(source, /--iv-app-background|--iv-glass-surface|--iv-surface-blur/);
  }
  assert.match(referenceEntry, /APP_SETTINGS_STORAGE_KEY/);
  assert.doesNotMatch(referenceEntry, /galleryStore|useGalleryStore/);
});
