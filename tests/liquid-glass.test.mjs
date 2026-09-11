import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test, { after } from "node:test";
import { createPinia, setActivePinia } from "pinia";
import { createServer } from "vite";

const vite = await createServer({
  appType: "custom",
  server: { middlewareMode: true },
});
const { APP_SETTINGS_STORAGE_KEY, createDefaultAppearanceSettings, useAppStore } = await vite.ssrLoadModule(
  "/src/app/appStore.ts",
);
const { useTheme } = await vite.ssrLoadModule("/src/shared/composables/useTheme.ts");

after(() => vite.close());

function createStorage(initialValue = null) {
  let value = initialValue;
  return {
    getItem: (key) => key === APP_SETTINGS_STORAGE_KEY ? value : null,
    setItem: (key, nextValue) => {
      if (key === APP_SETTINGS_STORAGE_KEY) value = nextValue;
    },
    saved: () => value,
  };
}

function createStore(saved) {
  const storage = createStorage(saved === undefined ? null : JSON.stringify(saved));
  globalThis.window = { localStorage: storage };
  setActivePinia(createPinia());
  const store = useAppStore();
  store.hydrate();
  return { store, storage };
}

test("Liquid Glass defaults to disabled for new and legacy settings", () => {
  assert.deepEqual(createDefaultAppearanceSettings().liquidGlass, { enabled: false, intensity: 50 });

  const { store } = createStore({
    theme: "dark",
    accentColor: "#D9823A",
  });
  assert.deepEqual(store.appearance.liquidGlass, { enabled: false, intensity: 50 });
});

test("Liquid Glass enabled state and intensity use existing Appearance persistence", () => {
  const { store, storage } = createStore({
    appearance: { liquidGlass: { enabled: true, intensity: 75 } },
  });

  assert.equal(store.appearance.liquidGlass.enabled, true);
  assert.equal(store.appearance.liquidGlass.intensity, 75);
  store.persist();

  const persisted = JSON.parse(storage.saved());
  assert.deepEqual(persisted.appearance.liquidGlass, { enabled: true, intensity: 75 });
  assert.equal("liquidGlass" in persisted, false);
});

test("Applying Appearance updates the root glass state and intensity tokens", () => {
  const properties = new Map();
  globalThis.document = {
    documentElement: {
      dataset: {},
      style: { setProperty: (name, value) => properties.set(name, value) },
    },
  };
  const { store } = createStore({
    appearance: {
      theme: "light",
      accentColor: "#8B68D9",
      liquidGlass: { enabled: true, intensity: 25 },
    },
  });

  useTheme().applyAppearance(store.appearance);

  assert.equal(document.documentElement.dataset.liquidGlass, "true");
  assert.equal(properties.get("--iv-glass-blur"), "10px");
  assert.equal(properties.get("--iv-glass-saturation"), "108%");
  assert.equal(properties.get("--iv-glass-surface-alpha"), "85%");
  assert.equal(properties.get("--iv-glass-strong-alpha"), "93%");

  store.appearance.liquidGlass.enabled = false;
  useTheme().applyAppearance(store.appearance);
  assert.equal(document.documentElement.dataset.liquidGlass, "false");
  assert.equal(properties.get("--iv-glass-blur"), "0px");
});

test("shared glass CSS has capability and transparency fallbacks without image effects", async () => {
  const [globals, tokens] = await Promise.all([
    readFile(new URL("../src/shared/styles/globals.scss", import.meta.url), "utf8"),
    readFile(new URL("../src/shared/styles/tokens.scss", import.meta.url), "utf8"),
  ]);

  assert.match(globals, /@supports \(backdrop-filter: blur\(1px\)\)/);
  assert.match(globals, /@supports not \(backdrop-filter: blur\(1px\)\)/);
  assert.match(globals, /prefers-reduced-transparency/);
  assert.match(globals, /-webkit-backdrop-filter/);
  assert.doesNotMatch(globals, /transition:\s*all/);
  for (const token of [
    "--iv-glass-bg",
    "--iv-glass-bg-strong",
    "--iv-glass-border",
    "--iv-glass-highlight",
    "--iv-glass-blur",
    "--iv-glass-saturation",
    "--iv-glass-shadow",
  ]) assert.match(tokens, new RegExp(token));
});

test("image-critical surfaces remain neutral while floating chrome can consume glass", async () => {
  const [viewer, preview, reference, thumbnail, referenceEntry] = await Promise.all([
    readFile(new URL("../src/features/viewer/ViewerPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/gallery/components/QuickPreviewOverlay.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/reference/ReferenceWindowPage.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/features/library/components/ThumbnailPreview.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/reference-main.ts", import.meta.url), "utf8"),
  ]);

  assert.match(viewer, /background-color: var\(--iv-viewer-matte-active-surface\)/);
  assert.match(preview, /background-color: var\(--iv-viewer-matte-auto-surface\)/);
  assert.match(reference, /background-color: var\(--iv-image-critical-surface\)/);
  assert.match(thumbnail, /background-color: var\(--iv-thumbnail-matte-surface\)/);
  assert.doesNotMatch(viewer, /class="viewer-page__surface[^"]*iv-glass-surface/);
  assert.doesNotMatch(preview, /class="quick-preview__surface[^"]*iv-glass-surface/);
  assert.doesNotMatch(reference, /class="reference-window__surface[^"]*iv-glass-surface/);
  assert.doesNotMatch(thumbnail, /class="thumbnail-preview[^"]*iv-glass-surface/);
  assert.doesNotMatch(viewer, /\.viewer-page__(?:surface|image)[^{]*\{[^}]*--iv-glass-/s);
  assert.doesNotMatch(preview, /\.quick-preview__(?:surface|image)[^{]*\{[^}]*--iv-glass-/s);
  assert.doesNotMatch(reference, /\.reference-window__(?:surface|image)[^{]*\{[^}]*--iv-glass-/s);
  assert.doesNotMatch(thumbnail, /\.thumbnail-preview(?: > img)?[^{]*\{[^}]*--iv-glass-/s);
  assert.match(preview, /button class=\"iv-glass-surface\"/);
  assert.match(reference, /reference-window__toolbar iv-glass-surface/);
  assert.match(reference, /reference-window__more-menu iv-glass-surface/);
  assert.doesNotMatch(referenceEntry, /galleryStore|useGalleryStore/);
});
