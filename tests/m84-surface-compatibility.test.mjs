import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

test("selected gallery thumbnails do not tint image pixels", async () => {
  const source = await read("src/features/gallery/components/GalleryItem.vue");
  assert.match(source, /gallery-item--selected \.gallery-item__preview[^{]*\{[\s\S]*border-color: var\(--iv-accent\)/);
  assert.doesNotMatch(source, /gallery-item--selected \.gallery-item__button::after/);
});

test("image-critical surfaces stay independent from appearance layers", async () => {
  const [viewer, preview, reference, thumbnail, tokens] = await Promise.all([
    read("src/features/viewer/ViewerPage.vue"),
    read("src/features/gallery/components/QuickPreviewOverlay.vue"),
    read("src/features/reference/ReferenceWindowPage.vue"),
    read("src/features/library/components/ThumbnailPreview.vue"),
    read("src/shared/styles/tokens.scss"),
  ]);

  assert.match(thumbnail, /background-color: var\(--iv-thumbnail-matte-surface\)/);
  assert.match(preview, /background-color: var\(--iv-viewer-matte-auto-surface\)/);
  assert.match(viewer, /--iv-viewer-matte-active-surface: var\(--iv-viewer-matte-auto-surface\)/);
  assert.match(viewer, /\.viewer-page--background-dark[\s\S]*--iv-viewer-matte-active-surface: var\(--iv-viewer-matte-surface\)/);
  assert.match(viewer, /\.viewer-page--background-light[\s\S]*--iv-viewer-matte-active-surface: var\(--iv-viewer-matte-surface-light\)/);
  assert.match(reference, /background-color: var\(--iv-image-critical-surface\)/);

  for (const source of [viewer, preview, reference, thumbnail]) {
    assert.doesNotMatch(source, /--iv-app-background|--iv-glass-surface|--iv-surface-blur|backdrop-filter/);
  }
  assert.match(tokens, /--iv-thumbnail-matte-surface: #1B1E23/);
  assert.match(tokens, /--iv-viewer-matte-surface: #181B20/);
  assert.match(tokens, /--iv-viewer-matte-surface-light: #E4E7EB/);
  assert.match(tokens, /--iv-thumbnail-matte-surface: #ECEEF1/);
  assert.match(tokens, /--iv-viewer-matte-auto-surface: var\(--iv-viewer-matte-surface-light\)/);
});

test("matte polish keeps contain sizing and image pixels untouched", async () => {
  const [thumbnail, preview, viewer, reference] = await Promise.all([
    read("src/features/library/components/ThumbnailPreview.vue"),
    read("src/features/gallery/components/QuickPreviewOverlay.vue"),
    read("src/features/viewer/ViewerPage.vue"),
    read("src/features/reference/ReferenceWindowPage.vue"),
  ]);

  for (const source of [thumbnail, preview, viewer, reference]) assert.match(source, /object-fit: contain/);
  assert.doesNotMatch(thumbnail, /filter:\s*blur|mix-blend-mode|background-blend-mode|dominant-color|color-mix.*image/);
  assert.doesNotMatch(preview, /filter:\s*blur|mix-blend-mode|background-blend-mode|dominant-color|color-mix.*image/);
  assert.doesNotMatch(viewer, /\.viewer-page(?:__surface|__image)[^{]*\{[^}]*filter:\s*blur/s);
  assert.doesNotMatch(viewer, /\.viewer-page(?:__surface|__image)[^{]*\{[^}]*mix-blend-mode/s);
  assert.doesNotMatch(reference, /\.reference-window(?:__surface|__image)[^{]*\{[^}]*filter:\s*blur/s);
  assert.doesNotMatch(reference, /\.reference-window(?:__surface|__image)[^{]*\{[^}]*mix-blend-mode/s);
});

test("light liquid glass defines readable shared overlay tokens", async () => {
  const globals = await read("src/shared/styles/globals.scss");
  assert.match(globals, /data-theme="light"]\[data-liquid-glass="true"]/);
  assert.match(globals, /--iv-overlay-text-primary: var\(--iv-text-primary\)/);
  assert.match(globals, /--iv-overlay-text-secondary: var\(--iv-text-secondary\)/);
  assert.match(globals, /--iv-overlay-accent-active: color-mix/);
});

test("information panel swatches use exact image colors without panel opacity", async () => {
  const source = await read("src/features/color/ImageInformationPanel.vue");
  assert.match(source, /:style="\{ backgroundColor: color\.hex \}/);
  const styles = source.slice(source.indexOf("<style"));
  assert.doesNotMatch(styles, /\.image-information\s*\{[^}]*opacity\s*:/s);
});
