import assert from "node:assert/strict";
import test from "node:test";
const registry = await import("../src/shared/shortcuts/shortcutRegistry.ts");
const dispatcherModule = await import("../src/shared/shortcuts/shortcutDispatcher.ts");
const escLayer = await import("../src/features/viewer/viewerEscLayer.ts");

function event(key, options = {}) {
  return {
    key,
    code: options.code ?? key,
    ctrlKey: options.ctrlKey ?? false,
    shiftKey: options.shiftKey ?? false,
    altKey: options.altKey ?? false,
  };
}

test("normalizes Ctrl, Shift, and Alt in a stable order", () => {
  assert.equal(registry.normalizeKeyCombination("Alt+Shift+Ctrl+a"), "Ctrl+Shift+Alt+A");
  assert.equal(registry.normalizeKeyCombination("ctrl+shift+alt+a"), "Ctrl+Shift+Alt+A");
  assert.equal(registry.normalizeShortcut(event("a", { code: "KeyA", ctrlKey: true, shiftKey: true, altKey: true })), "Ctrl+Shift+Alt+A");
  assert.equal(registry.normalizeShortcut(event(" ", { code: "Space" })), "Space");
});

test("conflicts are rejected only within the same scope", () => {
  assert.throws(() => registry.assertNoShortcutConflicts([
    { id: "a", keys: ["Space"], scope: "gallery", description: "a", category: "x" },
    { id: "b", keys: ["Space"], scope: "gallery", description: "b", category: "x" },
  ]), /Shortcut conflict/);
  assert.doesNotThrow(() => registry.assertNoShortcutConflicts([
    { id: "a", keys: ["Space"], scope: "gallery", description: "a", category: "x" },
    { id: "b", keys: ["Space"], scope: "viewer", description: "b", category: "x" },
  ]));
});

test("dispatcher isolates active scopes and guards editable targets", () => {
  globalThis.HTMLElement = class HTMLElement {
    constructor(editable = false) { this.closest = () => editable ? ({ }) : null; }
  };
  const dispatcher = new dispatcherModule.ShortcutDispatcher();
  const calls = [];
  const unregisterGallery = dispatcher.register("gallery", {
    "gallery.preview": () => { calls.push("gallery"); },
  });
  const keyboardEvent = { ...event(" ", { code: "Space" }), target: {}, repeat: false, preventDefault() { this.prevented = true; } };
  assert.equal(dispatcher.dispatch(keyboardEvent), true);
  assert.deepEqual(calls, ["gallery"]);
  assert.equal(keyboardEvent.prevented, true);

  const input = new globalThis.HTMLElement(true);
  const guardedEvent = { ...keyboardEvent, target: input, prevented: false, preventDefault() { this.prevented = true; } };
  assert.equal(dispatcher.dispatch(guardedEvent), false);
  assert.deepEqual(calls, ["gallery"]);

  const escapeCalls = [];
  const viewerArrowCalls = [];
  const unregisterViewer = dispatcher.register("viewer", {
    "viewer.escape": () => { escapeCalls.push("escape"); },
    "viewer.previous": () => { viewerArrowCalls.push("previous"); },
  });
  const viewerArrow = { ...event("ArrowLeft"), target: {}, repeat: false, preventDefault() {} };
  assert.equal(dispatcher.dispatch(viewerArrow), true);
  assert.deepEqual(viewerArrowCalls, ["previous"]);
  const gallerySpaceWhileViewerActive = { ...event(" ", { code: "Space" }), target: {}, repeat: false, preventDefault() {} };
  assert.equal(dispatcher.dispatch(gallerySpaceWhileViewerActive), false);
  assert.deepEqual(calls, ["gallery"]);
  const escapeEvent = { ...event("Escape"), target: input, repeat: false, preventDefault() {} };
  assert.equal(dispatcher.dispatch(escapeEvent), false);
  assert.deepEqual(escapeCalls, []);
  const consumedEscape = { ...event("Escape"), target: {}, repeat: false, defaultPrevented: true, preventDefault() {} };
  assert.equal(dispatcher.dispatch(consumedEscape), false);
  assert.deepEqual(escapeCalls, []);
  unregisterViewer();
  unregisterGallery();
});

test("viewer Escape layers resolve from nearest to route", () => {
  assert.equal(escLayer.getViewerEscapeLayer({ colorPickerOpen: true, informationPanelOpen: true, backgroundMenuOpen: true }), "colorPicker");
  assert.equal(escLayer.getViewerEscapeLayer({ colorPickerOpen: false, informationPanelOpen: true, backgroundMenuOpen: true }), "informationPanel");
  assert.equal(escLayer.getViewerEscapeLayer({ colorPickerOpen: false, informationPanelOpen: false, backgroundMenuOpen: true }), "backgroundMenu");
  assert.equal(escLayer.getViewerEscapeLayer({ colorPickerOpen: false, informationPanelOpen: false, backgroundMenuOpen: false }), "viewer");
});

test("registry preserves critical existing shortcut mappings", () => {
  const expected = new Map([
    ["gallery.preview", ["Space"]], ["gallery.viewer", ["Enter"]], ["gallery.selectAll", ["Ctrl+A"]], ["gallery.reference", ["B"]],
    ["viewer.previous", ["ArrowLeft"]], ["viewer.next", ["ArrowRight"]], ["viewer.fit", ["0"]], ["viewer.actualSize", ["1"]], ["viewer.fullscreen", ["F11"]], ["viewer.reference", ["B"]],
    ["reference.fit", ["0"]], ["reference.actualSize", ["1"]],
  ]);
  for (const [id, keys] of expected) assert.deepEqual(registry.shortcutDefinitions.find((definition) => definition.id === id)?.keys, keys);
});
