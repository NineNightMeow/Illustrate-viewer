import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const sidebar = read("src/features/settings/components/SettingsSidebar.vue");
const settings = read("src/features/settings/SettingsPage.vue");
const routes = read("src/app/routes.ts");
const appSidebar = read("src/shared/components/layout/Sidebar.vue");
const tauri = read("src-tauri/src/lib.rs");
const locales = ["src/i18n/en-US.ts", "src/i18n/zh-CN.ts", "src/i18n/ja-JP.ts"].map(read);

function localeLeafKeys(source) {
  const keys = [];
  const stack = [];
  for (const line of source.split(/\r?\n/)) {
    const match = line.match(/^(\s*)(?:"([^"]+)"|([A-Za-z0-9_]+)):\s*(.*)$/);
    if (!match) continue;

    const indent = match[1].length;
    while (stack.length && stack[stack.length - 1].indent >= indent) stack.pop();

    const key = match[2] ?? match[3];
    const value = match[4];
    if (value.startsWith("{")) stack.push({ indent, key });
    else keys.push([...stack.map((entry) => entry.key), key].join("."));
  }
  return keys.sort();
}

test("Settings exposes only Appearance, Shortcuts, and About", () => {
  assert.deepEqual([...sidebar.matchAll(/key: "([^"]+)"/g)].map((match) => match[1]), ["appearance", "shortcuts", "about"]);
  assert.doesNotMatch(settings, /settings-placeholder|appFont|Segoe UI/);
});

test("empty References navigation and route are removed", () => {
  assert.doesNotMatch(routes, /BootstrapView|path: "\/references"/);
  assert.doesNotMatch(appSidebar, /navigation.references|path: "\/references"/);
  assert.doesNotMatch(sidebar, /gallery|viewer|reference|library|advanced/);
});

test("About includes version, contributors, and repository links", () => {
  assert.match(settings, /app_version/);
  assert.match(settings, /github.com\/NineNightMeow/);
  assert.match(settings, /github.com\/HSP-hapy/);
  assert.match(settings, /github.com\/NineNightMeow\/Illustrate-viewer/);
  for (const locale of locales) {
    for (const key of ["version", "contributors", "repository", "openGitHubProfile", "openRepository", "shortcutCategories", "shortcutActions"]) {
      assert.match(locale, new RegExp(`${key}:`));
    }
  }
});

test("locale leaf key structures stay aligned", () => {
  const [reference, ...others] = locales.map(localeLeafKeys);
  for (const keys of others) assert.deepEqual(keys, reference);
});

test("shortcut action copy uses nested i18n keys", () => {
  for (const locale of locales) {
    assert.match(locale, /shortcutActions:\s*\{[\s\S]*global:\s*\{[\s\S]*searchFocus:/);
    assert.doesNotMatch(locale, /[\"'](?:global|gallery|viewer|reference)\.[A-Za-z]+[\"']\s*:/);
  }
});

test("visible copy avoids development placeholders", () => {
  for (const locale of locales) {
    assert.doesNotMatch(locale, /Coming soon|later batch|future version|将在后续|后续批次|占位|プレースホルダー/i);
  }
});

test("startup dialog keeps internal error details out of the user message", () => {
  assert.match(tauri, /eprintln!\(\"Illustrate Viewer startup failed: {error}/);
  assert.doesNotMatch(tauri, /local image metadata[^\n]*\{error\}/);
});

test("image and reference surfaces expose recovery-oriented copy", () => {
  const viewer = read("src/features/viewer/ViewerPage.vue");
  const quickPreview = read("src/features/gallery/components/QuickPreviewOverlay.vue");
  const reference = read("src/features/reference/ReferenceWindowPage.vue");
  for (const source of [viewer, quickPreview, reference]) assert.match(source, /sourceUnreadable|referenceImageFailed|referenceRestoreFailed/);
  assert.match(viewer, /:aria-label="\$t\('viewer\.reference'\)"[\s\S]*?:disabled="!currentAsset"[\s\S]*?@click="openCurrentReference"/);
});
