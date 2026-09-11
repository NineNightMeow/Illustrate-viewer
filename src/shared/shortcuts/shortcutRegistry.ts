export type ShortcutScope = "global" | "gallery" | "viewer" | "reference" | "search";

export type ShortcutDefinition = {
  id: string;
  keys: readonly string[];
  scope: ShortcutScope;
  description: string;
  category: string;
  allowInInput?: boolean;
  allowRepeat?: boolean;
};

const MODIFIERS = new Set(["Ctrl", "Shift", "Alt"]);

export function normalizeKeyCombination(value: string) {
  const parts = value.split("+").map((part) => part.trim()).filter(Boolean);
  const modifiers = new Set<string>();
  let key = "";

  for (const part of parts) {
    const normalized = normalizeKeyName(part);
    if (MODIFIERS.has(normalized)) modifiers.add(normalized);
    else key = normalized;
  }

  return [...["Ctrl", "Shift", "Alt"].filter((modifier) => modifiers.has(modifier)), key]
    .filter(Boolean)
    .join("+");
}

export function normalizeShortcut(event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "shiftKey" | "altKey">) {
  const key = keyNameFromEvent(event);
  const modifiers = [
    event.ctrlKey ? "Ctrl" : null,
    event.shiftKey ? "Shift" : null,
    event.altKey ? "Alt" : null,
  ].filter((modifier): modifier is string => modifier !== null);

  return [...modifiers, key].filter(Boolean).join("+");
}

export function isEditableTarget(target: EventTarget | null) {
  return typeof HTMLElement !== "undefined" && target instanceof HTMLElement
    && Boolean(target.closest("input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='textbox']"));
}

export function assertNoShortcutConflicts(definitions: ShortcutDefinition[]) {
  const seen = new Map<string, ShortcutDefinition>();

  for (const definition of definitions) {
    for (const key of definition.keys) {
      const normalized = normalizeKeyCombination(key);
      const identity = `${definition.scope}:${normalized}`;
      const previous = seen.get(identity);
      if (previous && previous.id !== definition.id) {
        throw new Error(
          `Shortcut conflict for ${identity}: ${previous.id} and ${definition.id}`,
        );
      }
      seen.set(identity, definition);
    }
  }
}

export function createShortcutRegistry(definitions: ShortcutDefinition[]) {
  const frozen = definitions.map((definition) => ({
    ...definition,
    keys: [...definition.keys],
  }));
  assertNoShortcutConflicts(frozen);
  return Object.freeze(frozen.map((definition) => Object.freeze(definition)));
}

export const shortcutDefinitions = createShortcutRegistry([
  {
    id: "global.searchFocus",
    keys: ["Ctrl+F"],
    scope: "global",
    description: "Focus the global Search field",
    category: "Global",
    allowInInput: true,
  },
  {
    id: "gallery.preview",
    keys: ["Space"],
    scope: "gallery",
    description: "Open or close Quick Preview",
    category: "Gallery",
  },
  {
    id: "gallery.viewer",
    keys: ["Enter"],
    scope: "gallery",
    description: "Open the full Viewer",
    category: "Gallery",
  },
  {
    id: "gallery.selectAll",
    keys: ["Ctrl+A"],
    scope: "gallery",
    description: "Select all visible gallery results",
    category: "Gallery",
  },
  {
    id: "gallery.thumbnailSize",
    keys: ["Ctrl+Wheel"],
    scope: "gallery",
    description: "Adjust thumbnail size",
    category: "Gallery",
  },
  {
    id: "gallery.reference",
    keys: ["B"],
    scope: "gallery",
    description: "Open the selected image in Reference",
    category: "Gallery",
    allowRepeat: false,
  },
  {
    id: "gallery.previewPrevious",
    keys: ["ArrowLeft"],
    scope: "gallery",
    description: "Show the previous image in Quick Preview",
    category: "Gallery",
  },
  {
    id: "gallery.previewNext",
    keys: ["ArrowRight"],
    scope: "gallery",
    description: "Show the next image in Quick Preview",
    category: "Gallery",
  },
  {
    id: "gallery.closePreview",
    keys: ["Escape"],
    scope: "gallery",
    description: "Close Quick Preview",
    category: "Gallery",
  },
  {
    id: "viewer.escape",
    keys: ["Escape"],
    scope: "viewer",
    description: "Close the nearest Viewer layer",
    category: "Viewer",
  },
  {
    id: "viewer.previous",
    keys: ["ArrowLeft"],
    scope: "viewer",
    description: "Show the previous image",
    category: "Viewer",
  },
  {
    id: "viewer.next",
    keys: ["ArrowRight"],
    scope: "viewer",
    description: "Show the next image",
    category: "Viewer",
  },
  {
    id: "viewer.fit",
    keys: ["0"],
    scope: "viewer",
    description: "Fit the image to the viewport",
    category: "Viewer",
  },
  {
    id: "viewer.actualSize",
    keys: ["1"],
    scope: "viewer",
    description: "Show the image at 100%",
    category: "Viewer",
  },
  {
    id: "viewer.fullscreen",
    keys: ["F11"],
    scope: "viewer",
    description: "Toggle fullscreen",
    category: "Viewer",
  },
  {
    id: "viewer.reference",
    keys: ["B"],
    scope: "viewer",
    description: "Open the current image in Reference",
    category: "Viewer",
    allowRepeat: false,
  },
  {
    id: "reference.fit",
    keys: ["0"],
    scope: "reference",
    description: "Fit the reference image",
    category: "Reference",
  },
  {
    id: "reference.actualSize",
    keys: ["1"],
    scope: "reference",
    description: "Show the reference image at 100%",
    category: "Reference",
  },
]);

export function formatShortcutKeys(definition: ShortcutDefinition) {
  return definition.keys.map(normalizeKeyCombination).join(" / ");
}

function keyNameFromEvent(event: Pick<KeyboardEvent, "key" | "code">) {
  if (event.code === "Space" || event.key === " ") return "Space";
  if (event.key.toLowerCase() === "escape" || event.key.toLowerCase() === "esc") return "Escape";
  if (event.key.length === 1 && /[a-z]/i.test(event.key)) return event.key.toUpperCase();
  if (event.code.startsWith("Key") && event.code.length === 4) return event.code.slice(3).toUpperCase();
  return normalizeKeyName(event.key);
}

function normalizeKeyName(value: string) {
  const key = value.trim();
  if (key === " " || key.toLowerCase() === "spacebar") return "Space";
  if (key.toLowerCase() === "esc") return "Escape";
  if (key.toLowerCase() === "ctrl") return "Ctrl";
  if (key.toLowerCase() === "control") return "Ctrl";
  if (key.toLowerCase() === "shift") return "Shift";
  if (key.toLowerCase() === "alt") return "Alt";
  if (key.length === 1 && /[a-z]/i.test(key)) return key.toUpperCase();
  return key;
}
