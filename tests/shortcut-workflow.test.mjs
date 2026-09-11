import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const registry = await import("../src/shared/shortcuts/shortcutRegistry.ts");
const dispatcherModule = await import("../src/shared/shortcuts/shortcutDispatcher.ts");
const grid = await import("../src/features/gallery/virtualGrid.ts");

function event(key, options = {}) {
  return {
    key,
    code: options.code ?? key,
    ctrlKey: options.ctrlKey ?? false,
    shiftKey: options.shiftKey ?? false,
    altKey: options.altKey ?? false,
    repeat: options.repeat ?? false,
    target: options.target ?? {},
    defaultPrevented: options.defaultPrevented ?? false,
    preventDefault() { this.defaultPrevented = true; },
  };
}

test("Gallery thumbnail size has bounded Ctrl+Wheel steps", () => {
  assert.equal(grid.adjustThumbnailSize(160, -100), 180);
  assert.equal(grid.adjustThumbnailSize(grid.MAX_THUMBNAIL_SIZE, -100), grid.MAX_THUMBNAIL_SIZE);
  assert.equal(grid.adjustThumbnailSize(grid.MIN_THUMBNAIL_SIZE, 100), grid.MIN_THUMBNAIL_SIZE);
  assert.equal(registry.shortcutDefinitions.find((item) => item.id === "gallery.thumbnailSize")?.keys[0], "Ctrl+Wheel");
});

test("Gallery Ctrl+Wheel leaves ordinary wheel and editable targets alone", async () => {
  globalThis.HTMLElement = class HTMLElement {
    constructor(editable = false) { this.closest = () => editable ? ({}) : null; }
  };
  const { shouldResizeThumbnails } = await import("../src/features/gallery/virtualGrid.ts");
  assert.equal(shouldResizeThumbnails({ ctrlKey: false, deltaY: 100, target: {} }), false);
  assert.equal(shouldResizeThumbnails({ ctrlKey: true, deltaY: 0, target: {} }), false);
  assert.equal(shouldResizeThumbnails({ ctrlKey: true, deltaY: 100, target: new globalThis.HTMLElement(true) }), false);
  assert.equal(shouldResizeThumbnails({ ctrlKey: true, deltaY: 100, target: new globalThis.HTMLElement(false) }), true);
});

test("global Ctrl+F is allowed from an input while page shortcuts remain guarded", () => {
  globalThis.HTMLElement = class HTMLElement {
    constructor(editable = false) { this.closest = () => editable ? ({}) : null; }
  };
  const dispatcher = new dispatcherModule.ShortcutDispatcher();
  const calls = [];
  const unregisterGlobal = dispatcher.register("global", {
    "global.searchFocus": () => { calls.push("search"); },
  });
  const unregisterGallery = dispatcher.register("gallery", {
    "gallery.preview": () => { calls.push("preview"); },
  });

  const input = new globalThis.HTMLElement(true);
  assert.equal(dispatcher.dispatch(event("f", { code: "KeyF", ctrlKey: true, target: input })), true);
  assert.deepEqual(calls, ["search"]);
  assert.equal(dispatcher.dispatch(event(" ", { code: "Space", target: input })), false);
  assert.deepEqual(calls, ["search"]);

  unregisterGallery();
  unregisterGlobal();
});

test("editable fields do not trigger common page shortcuts", () => {
  globalThis.HTMLElement = class HTMLElement {
    constructor() { this.closest = () => ({}); }
  };
  const dispatcher = new dispatcherModule.ShortcutDispatcher();
  const calls = [];
  const unregister = dispatcher.register("viewer", {
    "viewer.previous": () => { calls.push("previous"); },
    "viewer.actualSize": () => { calls.push("actual-size"); },
    "viewer.reference": () => { calls.push("reference"); },
  });
  const target = new globalThis.HTMLElement();
  for (const [key, code] of [["ArrowLeft", "ArrowLeft"], ["0", "Digit0"], ["1", "Digit1"], ["b", "KeyB"]]) {
    assert.equal(dispatcher.dispatch(event(key, { code, target })), false);
  }
  assert.deepEqual(calls, []);
  unregister();
});

test("scope-specific bindings stay isolated and registry remains conflict-free", () => {
  const gallery = registry.shortcutDefinitions.find((item) => item.id === "gallery.preview");
  const viewer = registry.shortcutDefinitions.find((item) => item.id === "viewer.previous");
  assert.equal(gallery?.scope, "gallery");
  assert.equal(viewer?.scope, "viewer");
  assert.doesNotThrow(() => registry.assertNoShortcutConflicts([...registry.shortcutDefinitions]));
});

test("Settings shortcut page reads the registry instead of duplicating bindings", async () => {
  const source = await readFile(new URL("../src/features/settings/SettingsPage.vue", import.meta.url), "utf8");
  assert.match(source, /formatShortcutKeys, shortcutDefinitions/);
  assert.match(source, /shortcutDefinitions/);
  assert.doesNotMatch(source, /Ctrl\+A|F11|>B<|\"B\"/);
});
