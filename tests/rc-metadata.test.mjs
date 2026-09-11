import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const read = (path) => fs.readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const readJson = (path) => JSON.parse(read(path));

test("release metadata uses the formal product identity", () => {
  const packageJson = readJson("package.json");
  const cargo = read("src-tauri/Cargo.toml");
  const tauri = readJson("src-tauri/tauri.conf.json");
  const index = read("index.html");
  const reference = read("reference.html");
  const about = read("src/features/settings/SettingsPage.vue");
  const readme = read("README.md");

  assert.equal(packageJson.version, "0.1.0");
  assert.equal(packageJson.description, "Illustrate Viewer desktop image library");
  assert.equal(tauri.productName, "Illustrate Viewer");
  assert.equal(tauri.version, "0.1.0");
  assert.equal(tauri.identifier, "com.illustrateviewer.desktop");
  assert.equal(tauri.app.windows[0].title, "Illustrate Viewer");
  assert.match(cargo, /name = "illustrate-viewer"/);
  assert.match(cargo, /version = "0.1.0"/);
  assert.match(index, /<title>Illustrate Viewer<\/title>/);
  assert.match(reference, /<title>Illustrate Viewer Reference Window<\/title>/);
  assert.ok(about.includes("$t('app.name')"));
  assert.match(readme, /Illustrate Viewer/);
  for (const localePath of ["src/i18n/en-US.ts", "src/i18n/zh-CN.ts", "src/i18n/ja-JP.ts"]) {
    assert.match(read(localePath), /name: "Illustrate Viewer"/);
  }
  for (const source of [packageJson.repository, readme, cargo]) assert.match(String(source), /github.com\/NineNightMeow\/Illustrate-viewer/);
});

test("About presents localized product details and real credits", () => {
  const about = read("src/features/settings/SettingsPage.vue");
  assert.match(about, /settings\.productDescription/);
  assert.match(about, /settings\.license/);
  assert.match(about, /settings\.contributorNineNightMeow/);
  assert.match(about, /settings\.contributorHspHapy/);
  assert.match(about, /settings\.repositoryName/);
  for (const localePath of ["src/i18n/en-US.ts", "src/i18n/zh-CN.ts", "src/i18n/ja-JP.ts"]) {
    const locale = read(localePath);
  for (const key of ["productDescription", "contributorNineNightMeow", "contributorHspHapy", "repositoryName", "licenseLabel", "license", "openGitHubProfileFor"]) {
      assert.match(locale, new RegExp(`${key}:`));
    }
  }
});

test("MIT license metadata is present", () => {
  assert.match(read("LICENSE"), /^MIT License/m);
  assert.match(read("package.json"), /"license": "MIT"/);
  assert.match(read("src-tauri/Cargo.toml"), /license = "MIT"/);
  const tauri = readJson("src-tauri/tauri.conf.json");
  assert.equal(tauri.bundle.license, "MIT");
  assert.equal(tauri.bundle.licenseFile, "../LICENSE");
});

test("deleted References route has no navigation or shortcut residue", () => {
  const sources = [
    read("src/app/routes.ts"),
    read("src/shared/components/layout/Sidebar.vue"),
    read("src/shared/shortcuts/shortcutDispatcher.ts"),
  ];
  for (const source of sources) {
    assert.doesNotMatch(source, /\/references|navigation\.references|BootstrapView/);
  }
  assert.ok(fs.existsSync(new URL("../src/features/reference/ReferenceWindowPage.vue", import.meta.url)));
  assert.ok(fs.existsSync(new URL("../src/features/reference/referenceWindowApi.ts", import.meta.url)));
});

test("shortcut settings uses explicit scope i18n mapping", () => {
  const settings = read("src/features/settings/SettingsPage.vue");
  assert.match(settings, /shortcutCategoryKeys/);
  assert.doesNotMatch(settings, /category\.toLowerCase()/);
  assert.doesNotMatch(settings, /definition\.category/);
});
