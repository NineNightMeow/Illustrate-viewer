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

test("custom background persists only its local source reference and settings", () => {
  const { store, storage } = hydrateSettings({
    appearance: {
      customBackground: {
        enabled: true,
        source: "C:\\\\Wallpapers\\studio.png",
        fit: "contain",
        opacity: 72,
        blur: 12,
        dim: 38,
      },
    },
  });

  assert.deepEqual(store.appearance.customBackground, {
    enabled: true,
    source: "C:\\\\Wallpapers\\studio.png",
    fit: "contain",
    opacity: 72,
    blur: 12,
    dim: 38,
  });

  store.persist();
  const persisted = JSON.parse(storage.saved());
  assert.deepEqual(persisted.appearance.customBackground, store.appearance.customBackground);
  assert.equal(JSON.stringify(persisted).includes("data:image"), false);
  assert.equal("customBackgroundSourceStatus" in persisted, false);
});

test("invalid background sources and controls fall back to safe defaults", () => {
  const { store } = hydrateSettings({
    appearance: {
      customBackground: {
        enabled: true,
        source: "https://example.test/background.png",
        fit: "stretch",
        opacity: 101,
        blur: 25,
        dim: -1,
      },
    },
  });

  assert.deepEqual(store.appearance.customBackground, {
    enabled: true,
    source: null,
    fit: "cover",
    opacity: 100,
    blur: 0,
    dim: 0,
  });
});

test("disabled and removed backgrounds do not retain active rendering state", () => {
  const { store } = hydrateSettings({
    appearance: {
      customBackground: {
        enabled: false,
        source: "C:\\\\Wallpapers\\studio.png",
      },
    },
  });

  assert.equal(store.appearance.customBackground.enabled, false);
  assert.equal(store.appearance.customBackground.source, "C:\\\\Wallpapers\\studio.png");

  store.setCustomBackgroundSourceStatus("unavailable");
  store.clearCustomBackground();

  assert.deepEqual(store.appearance.customBackground, {
    enabled: false,
    source: null,
    fit: "cover",
    opacity: 100,
    blur: 0,
    dim: 0,
  });
  assert.equal(store.customBackgroundSourceStatus, "idle");
});

test("wallpaper rendering stays main-window-only and requires a confirmed source", async () => {
  const [app, layer, referenceEntry, nativeCommand] = await Promise.all([
    readFile(new URL("../src/App.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/app/CustomBackgroundLayer.vue", import.meta.url), "utf8"),
    readFile(new URL("../src/reference-main.ts", import.meta.url), "utf8"),
    readFile(new URL("../src-tauri/src/commands/background.rs", import.meta.url), "utf8"),
  ]);

  assert.match(app, /customBackgroundSourceStatus === 'available'/);
  assert.match(app, /--iv-content-surface: color-mix\(in srgb, var\(--iv-bg-primary\) 20%, transparent\)/);
  assert.match(app, /--iv-chrome-surface: color-mix\(in srgb, var\(--iv-bg-primary\) 56%, transparent\)/);
  assert.match(app, /--iv-chrome-sidebar-surface: color-mix\(in srgb, var\(--iv-bg-secondary\) 56%, transparent\)/);
  assert.match(app, /has-custom-background > \.app-shell\s*\{\s*background-color: transparent;/);
  assert.match(layer, /props\.enabled && Boolean\(resolvedSource\.value\) && !imageFailed\.value/);
  assert.match(layer, /filter: blur\(var\(--iv-background-image-blur\)\)/);
  assert.doesNotMatch(layer, /backdrop-filter/);
  assert.doesNotMatch(referenceEntry, /CustomBackgroundLayer/);

  assert.ok(nativeCommand.includes('add_filter("Image", &["png", "jpg", "jpeg", "webp"])'));
  assert.match(nativeCommand, /is_approved_source\(&app, &source\)/);
  assert.match(nativeCommand, /asset_protocol_scope\(\)\s*\.allow_file\(&source\)/s);
});
