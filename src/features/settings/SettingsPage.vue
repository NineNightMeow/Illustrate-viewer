<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "vue-i18n";
import { useAppStore, type AppTheme, type CustomBackgroundFit } from "../../app/appStore";
import { i18n, type AppLocale } from "../../i18n";
import AppColorSwatch from "../../shared/components/ui/AppColorSwatch.vue";
import AppSelect from "../../shared/components/ui/AppSelect.vue";
import AppToggle from "../../shared/components/ui/AppToggle.vue";
import SegmentedControl from "../../shared/components/ui/SegmentedControl.vue";
import SettingsRow from "./components/SettingsRow.vue";
import SettingsSidebar from "./components/SettingsSidebar.vue";
import AppButton from "../../shared/components/ui/AppButton.vue";
import { chooseCustomBackground, clearCustomBackground } from "./customBackgroundApi";
import { formatShortcutKeys, shortcutDefinitions, type ShortcutScope } from "../../shared/shortcuts/shortcutRegistry";
import logoUrl from "../../../logos/logo.png";

const { t } = useI18n();
const appStore = useAppStore();
const activeCategory = ref("appearance");
const appVersion = ref("0.1.0");
const shortcutCategoryKeys: Record<Exclude<ShortcutScope, "search">, string> = {
  global: "global",
  gallery: "gallery",
  viewer: "viewer",
  reference: "reference",
};
const shortcutGroups = computed(() => {
  const groups = new Map<Exclude<ShortcutScope, "search">, (typeof shortcutDefinitions)[number][]>();
  for (const definition of shortcutDefinitions) {
    if (!(definition.scope in shortcutCategoryKeys)) continue;
    const scope = definition.scope as Exclude<ShortcutScope, "search">;
    const group = groups.get(scope) ?? [];
    group.push(definition);
    groups.set(scope, group);
  }
  return [...groups.entries()].map(([scope, definitions]) => ({
    scope,
    definitions,
    labelKey: `settings.shortcutCategories.${shortcutCategoryKeys[scope]}`,
  }));
});
const shortcutDisplayKeyMap: Record<string, string> = {
  Wheel: "wheel",
  ArrowLeft: "arrowLeft",
  ArrowRight: "arrowRight",
  ArrowUp: "arrowUp",
  ArrowDown: "arrowDown",
  Ctrl: "control",
  Control: "control",
  Shift: "shift",
  Alt: "alt",
};

function formatDisplayedShortcut(definition: (typeof shortcutDefinitions)[number]) {
  return formatShortcutKeys(definition)
    .split("+")
    .map((key) => {
      const translationKey = shortcutDisplayKeyMap[key];
      return translationKey ? t(`settings.shortcutKeys.${translationKey}`) : key;
    })
    .join("+");
}
const accentOptions = ["blue", "purple", "rose", "orange", "yellow", "green", "cyan"] as const;

const theme = computed<AppTheme>({
  get: () => appStore.appearance.theme,
  set: (value) => {
    appStore.appearance.theme = value;
  },
});

const liquidGlassEnabled = computed({
  get: () => appStore.appearance.liquidGlass.enabled,
  set: (value: boolean) => {
    appStore.appearance.liquidGlass.enabled = value;
  },
});

type LiquidGlassIntensity = "low" | "medium" | "high";

const liquidGlassIntensity = computed<LiquidGlassIntensity>({
  get: () => appStore.appearance.liquidGlass.intensity <= 37
    ? "low"
    : appStore.appearance.liquidGlass.intensity <= 62
      ? "medium"
      : "high",
  set: (value) => {
    appStore.appearance.liquidGlass.intensity = value === "low" ? 25 : value === "medium" ? 50 : 75;
  },
});

const customBackgroundEnabled = computed({
  get: () => appStore.appearance.customBackground.enabled,
  set: (value: boolean) => {
    appStore.appearance.customBackground.enabled = value;
  },
});

const customBackgroundFit = computed<CustomBackgroundFit>({
  get: () => appStore.appearance.customBackground.fit,
  set: (value) => {
    appStore.appearance.customBackground.fit = value;
  },
});

const customBackgroundOpacity = computed({
  get: () => appStore.appearance.customBackground.opacity,
  set: (value: number) => {
    appStore.appearance.customBackground.opacity = clampBackgroundControl(value, 0, 100);
  },
});

const customBackgroundBlur = computed({
  get: () => appStore.appearance.customBackground.blur,
  set: (value: number) => {
    appStore.appearance.customBackground.blur = clampBackgroundControl(value, 0, 24);
  },
});

const customBackgroundDim = computed({
  get: () => appStore.appearance.customBackground.dim,
  set: (value: number) => {
    appStore.appearance.customBackground.dim = clampBackgroundControl(value, 0, 100);
  },
});

const customBackgroundFitOptions = computed(() => [
  { value: "cover", label: t("settings.backgroundFitFill") },
  { value: "contain", label: t("settings.backgroundFitFit") },
]);

const customBackgroundFilename = computed(() => {
  const source = appStore.appearance.customBackground.source;
  const segments = source?.split(/[\\/]/);
  return segments?.[segments.length - 1] ?? null;
});

const customBackgroundUnavailable = computed(() => appStore.customBackgroundSourceStatus === "unavailable");
const isChoosingCustomBackground = ref(false);
const customBackgroundPickerError = ref(false);
const language = computed<AppLocale>({
  get: () => appStore.language,
  set: (value) => {
    appStore.language = value;
    i18n.global.locale.value = value;
  },
});

const themeOptions = computed(() => [
  { value: "system", label: t("settings.system") },
  { value: "light", label: t("settings.light") },
  { value: "dark", label: t("settings.dark") },
]);

const liquidGlassIntensityOptions = computed(() => [
  { value: "low", label: t("settings.liquidGlassLow") },
  { value: "medium", label: t("settings.liquidGlassMedium") },
  { value: "high", label: t("settings.liquidGlassHigh") },
]);

const languageOptions = computed(() => [
  { value: "zh-CN", label: t("settings.languageChinese") },
  { value: "ja-JP", label: t("settings.languageJapanese") },
  { value: "en-US", label: t("settings.languageEnglish") },
]);

const accentStyle = computed(() => ({
  backgroundColor: appStore.appearance.accentColor,
}));

const accentValues = computed(() => {
  if (typeof document === "undefined") return {} as Record<(typeof accentOptions)[number], string>;

  const styles = getComputedStyle(document.documentElement);
  return Object.fromEntries(
    accentOptions.map((option) => [
      option,
      styles.getPropertyValue(`--iv-accent-option-${option}`).trim(),
    ]),
  ) as Record<(typeof accentOptions)[number], string>;
});

function selectAccent(option: (typeof accentOptions)[number]) {
  const value = accentValues.value[option];
  if (value) appStore.appearance.accentColor = value;
}

onMounted(() => {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    void invoke<string>("app_version").then((version) => { appVersion.value = version; });
  }
});

function openExternal(url: string) {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) void openUrl(url);
  else window.open(url, "_blank", "noopener,noreferrer");
}

function isAccentSelected(option: (typeof accentOptions)[number]) {
  return accentValues.value[option].toUpperCase() === appStore.appearance.accentColor.toUpperCase();
}

function clampBackgroundControl(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, Math.round(value)));
}

function updateCustomBackgroundControl(
  control: "opacity" | "blur" | "dim",
  event: Event,
) {
  const input = event.target;
  if (!(input instanceof HTMLInputElement)) return;

  if (control === "opacity") customBackgroundOpacity.value = Number(input.value);
  else if (control === "blur") customBackgroundBlur.value = Number(input.value);
  else customBackgroundDim.value = Number(input.value);
}

async function selectCustomBackground() {
  if (isChoosingCustomBackground.value) return;

  isChoosingCustomBackground.value = true;
  customBackgroundPickerError.value = false;
  try {
    const selected = await chooseCustomBackground();
    if (!selected) return;

    appStore.setCustomBackgroundSource(selected.path);
    appStore.appearance.customBackground.enabled = true;
  } catch {
    customBackgroundPickerError.value = true;
  } finally {
    isChoosingCustomBackground.value = false;
  }
}

async function removeCustomBackground() {
  appStore.clearCustomBackground();
  customBackgroundPickerError.value = false;
  try {
    await clearCustomBackground();
  } catch {
    // The local setting is still cleared; a later choose operation replaces stale approval.
  }
}
</script>

<template>
  <div class="settings-page">
    <div class="settings-page__inner">
      <SettingsSidebar v-model="activeCategory" />

      <section class="settings-content" aria-labelledby="settings-page-title">
        <header class="settings-content__header">
          <h1 id="settings-page-title">{{ $t(`settings.${activeCategory}`) }}</h1>
        </header>

        <section v-if="activeCategory === 'appearance'" class="settings-panel iv-glass-surface">
          <SettingsRow :title="$t('settings.theme')">
            <SegmentedControl v-model="theme" :options="themeOptions" />
          </SettingsRow>

          <SettingsRow :title="$t('settings.languageLabel')">
            <AppSelect v-model="language" :options="languageOptions" :aria-label="$t('settings.languageLabel')" />
          </SettingsRow>

          <SettingsRow :title="$t('settings.accentColor')">
            <div class="accent-value">
              <div class="accent-picker" role="group" :aria-label="$t('settings.accentColor')">
                <AppColorSwatch
                  v-for="option in accentOptions"
                  :key="option"
                  :color="`var(--iv-accent-option-${option})`"
                  :label="$t('settings.accentColor')"
                  :selected="isAccentSelected(option)"
                  @click="selectAccent(option)"
                />
              </div>
              <span class="accent-value__preview" :style="accentStyle" aria-hidden="true" />
              <span>{{ appStore.appearance.accentColor.toUpperCase() }}</span>
            </div>
          </SettingsRow>

          <SettingsRow
            :title="$t('settings.liquidGlass')"
            :description="$t('settings.liquidGlassDescription')"
          >
            <div class="setting-toggle">
              <AppToggle v-model="liquidGlassEnabled" :label="$t('settings.liquidGlass')" />
              <span>{{ liquidGlassEnabled ? $t("settings.on") : $t("settings.off") }}</span>
            </div>
          </SettingsRow>

          <SettingsRow :title="$t('settings.liquidGlassIntensity')" :disabled="!liquidGlassEnabled">
            <SegmentedControl
              v-model="liquidGlassIntensity"
              :options="liquidGlassIntensityOptions"
              :disabled="!liquidGlassEnabled"
            />
          </SettingsRow>

          <SettingsRow
            :title="$t('settings.customBackground')"
            :description="$t('settings.customBackgroundDescription')"
          >
            <div class="setting-toggle">
              <AppToggle
                v-model="customBackgroundEnabled"
                :label="$t('settings.customBackground')"
              />
              <span>{{ customBackgroundEnabled ? $t("settings.on") : $t("settings.off") }}</span>
            </div>
          </SettingsRow>

          <SettingsRow :title="$t('settings.background')">
            <div class="background-source">
              <span
                v-if="customBackgroundFilename"
                class="background-source__filename"
                :class="{ 'is-unavailable': customBackgroundUnavailable }"
                :title="customBackgroundFilename"
              >
                {{ customBackgroundUnavailable ? $t("settings.backgroundUnavailable") : customBackgroundFilename }}
              </span>
              <AppButton variant="secondary" :disabled="isChoosingCustomBackground" @click="selectCustomBackground">
                {{ customBackgroundFilename ? $t("settings.backgroundChange") : $t("settings.backgroundChoose") }}
              </AppButton>
              <AppButton
                v-if="customBackgroundFilename"
                variant="ghost"
                @click="removeCustomBackground"
              >
                {{ $t("settings.backgroundRemove") }}
              </AppButton>
            </div>
          </SettingsRow>

          <SettingsRow :title="$t('settings.backgroundFit')" :disabled="!customBackgroundFilename">
            <SegmentedControl
              v-model="customBackgroundFit"
              :options="customBackgroundFitOptions"
              :disabled="!customBackgroundFilename"
            />
          </SettingsRow>

          <SettingsRow :title="$t('settings.backgroundOpacity', { value: customBackgroundOpacity })" :disabled="!customBackgroundFilename">
            <input
              class="background-range"
              type="range"
              min="0"
              max="100"
              step="1"
              :value="customBackgroundOpacity"
              :disabled="!customBackgroundFilename"
              :aria-label="$t('settings.backgroundOpacity', { value: customBackgroundOpacity })"
              @input="updateCustomBackgroundControl('opacity', $event)"
            >
          </SettingsRow>

          <SettingsRow :title="$t('settings.backgroundBlur', { value: customBackgroundBlur })" :disabled="!customBackgroundFilename">
            <input
              class="background-range"
              type="range"
              min="0"
              max="24"
              step="1"
              :value="customBackgroundBlur"
              :disabled="!customBackgroundFilename"
              :aria-label="$t('settings.backgroundBlur', { value: customBackgroundBlur })"
              @input="updateCustomBackgroundControl('blur', $event)"
            >
          </SettingsRow>

          <SettingsRow :title="$t('settings.backgroundDim', { value: customBackgroundDim })" :disabled="!customBackgroundFilename">
            <input
              class="background-range"
              type="range"
              min="0"
              max="100"
              step="1"
              :value="customBackgroundDim"
              :disabled="!customBackgroundFilename"
              :aria-label="$t('settings.backgroundDim', { value: customBackgroundDim })"
              @input="updateCustomBackgroundControl('dim', $event)"
            >
          </SettingsRow>

          <p v-if="customBackgroundPickerError" class="background-error" role="status">
            {{ $t("settings.backgroundPickerError") }}
          </p>
        </section>

        <section v-else-if="activeCategory === 'shortcuts'" class="shortcut-list" :aria-label="$t('settings.shortcuts')">
          <div v-for="category in shortcutGroups" :key="category.scope" class="shortcut-list__group">
            <h2>{{ $t(category.labelKey) }}</h2>
            <div class="shortcut-list__rows">
              <div v-for="definition in category.definitions" :key="definition.id" class="shortcut-list__row">
                <span class="shortcut-list__action">{{ $t(`settings.shortcutActions.${definition.id}`) }}</span>
                <kbd>{{ formatDisplayedShortcut(definition) }}</kbd>
              </div>
            </div>
          </div>
        </section>

        <section v-else-if="activeCategory === 'about'" class="about-panel iv-glass-surface" :aria-label="$t('settings.about')">
          <div class="about-panel__identity">
            <img class="about-panel__logo" :src="logoUrl" alt="" aria-hidden="true">
            <div>
              <h2>{{ $t('app.name') }}</h2>
              <p>{{ $t('settings.version') }} {{ appVersion }}</p>
              <p>{{ $t('settings.productDescription') }}</p>
            </div>
          </div>
          <div class="about-panel__section">
            <h3>{{ $t('settings.contributors') }}</h3>
            <a href="https://github.com/NineNightMeow" :aria-label="$t('settings.openGitHubProfileFor', { name: $t('settings.contributorNineNightMeow') })" @click.prevent="openExternal('https://github.com/NineNightMeow')">{{ $t('settings.contributorNineNightMeow') }}</a>
            <a href="https://github.com/HSP-hapy" :aria-label="$t('settings.openGitHubProfileFor', { name: $t('settings.contributorHspHapy') })" @click.prevent="openExternal('https://github.com/HSP-hapy')">{{ $t('settings.contributorHspHapy') }}</a>
          </div>
          <div class="about-panel__section">
            <h3>{{ $t('settings.repository') }}</h3>
            <a href="https://github.com/NineNightMeow/Illustrate-viewer" :aria-label="$t('settings.openRepository')" @click.prevent="openExternal('https://github.com/NineNightMeow/Illustrate-viewer')">{{ $t('settings.repositoryName') }}</a>
          </div>
          <div class="about-panel__section">
            <h3>{{ $t('settings.licenseLabel') }}</h3>
            <p>{{ $t('settings.license') }}</p>
          </div>
        </section>
      </section>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-page {
  min-height: 100%;
  padding: var(--iv-content-padding);
}

.settings-page__inner {
  display: grid;
  grid-template-columns: var(--iv-settings-nav-width) minmax(0, 1fr);
  gap: var(--iv-space-6);
  width: 100%;
  max-width: var(--iv-content-max-width);
}

.settings-content {
  min-width: 0;
}

.settings-content__header {
  padding-bottom: var(--iv-space-4);
}

.settings-content__header h1 {
  margin: 0;
  font-size: var(--iv-font-size-24);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-32);
}

.settings-panel {
  overflow: hidden;
  background-color: var(--iv-content-panel-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.background-source {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--iv-space-2);
  min-width: 0;
}

.background-source__filename {
  max-width: 240px;
  overflow: hidden;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.background-source__filename.is-unavailable,
.background-error {
  color: var(--iv-danger);
}

.background-range {
  width: var(--iv-settings-range-width);
  max-width: 100%;
  accent-color: var(--iv-accent);
}

.background-error {
  padding: 0 var(--iv-space-4) var(--iv-space-3);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.accent-picker {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
}

.accent-value,
.setting-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--iv-space-2);
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  line-height: var(--iv-line-height-18);
}

.accent-value__preview {
  display: inline-block;
  width: 24px;
  height: 24px;
  border: 2px solid var(--iv-text-primary);
  border-radius: var(--iv-radius-round);
  box-shadow: 0 0 0 var(--iv-focus-ring-width) var(--iv-accent-soft);
}

.setting-toggle {
  justify-content: flex-end;
  min-width: 0;
}

.about-panel {
  display: grid;
  gap: var(--iv-space-6);
  padding: var(--iv-space-6);
  background-color: var(--iv-content-panel-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.about-panel__identity h2,
.about-panel__identity p,
.about-panel__section h3,
.about-panel__section p {
  margin: 0;
}

.about-panel__identity {
  display: flex;
  align-items: center;
  gap: var(--iv-space-3);
}

.about-panel__identity > div {
  display: grid;
  gap: var(--iv-space-1);
}

.about-panel__logo {
  flex: 0 0 auto;
  width: 48px;
  height: 48px;
  object-fit: contain;
  border-radius: var(--iv-radius-lg);
}

.about-panel__identity h2 {
  font-size: var(--iv-font-size-20);
  line-height: var(--iv-line-height-28);
}

.about-panel__identity p {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
}

.about-panel__section p {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.about-panel__section {
  display: grid;
  gap: var(--iv-space-2);
}

.about-panel__section h3 {
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-13);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-18);
  text-transform: uppercase;
}

.about-panel__section a {
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--iv-accent);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.shortcut-list {
  display: grid;
  gap: var(--iv-space-5);
}

.shortcut-list__group {
  overflow: hidden;
  background-color: var(--iv-content-panel-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-md);
}

.shortcut-list__group h2 {
  margin: 0;
  padding: var(--iv-space-3) var(--iv-space-4);
  color: var(--iv-text-primary);
  font-size: var(--iv-font-size-15);
  font-weight: var(--iv-font-weight-semibold);
  line-height: var(--iv-line-height-20);
  background-color: var(--iv-surface);
  border-bottom: 1px solid var(--iv-border-subtle);
}

.shortcut-list__rows {
  display: grid;
}

.shortcut-list__row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--iv-space-4);
  min-height: var(--iv-control-height);
  padding: var(--iv-space-2) var(--iv-space-4);
  border-bottom: 1px solid var(--iv-border-subtle);
}

.shortcut-list__row:last-child {
  border-bottom: 0;
}

.shortcut-list__action {
  min-width: 0;
  color: var(--iv-text-secondary);
  font-size: var(--iv-font-size-14);
  line-height: var(--iv-line-height-20);
}

.shortcut-list kbd {
  min-width: 64px;
  padding: var(--iv-space-1) var(--iv-space-2);
  color: var(--iv-text-primary);
  font-family: inherit;
  font-size: var(--iv-font-size-12);
  line-height: var(--iv-line-height-16);
  text-align: center;
  background-color: var(--iv-surface);
  border: 1px solid var(--iv-border-subtle);
  border-radius: var(--iv-radius-xs);
}

@media (max-width: 900px) {
  .settings-page__inner {
    grid-template-columns: 1fr;
  }

  .settings-sidebar {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    width: 100%;
  }
}

@media (max-width: 640px) {
  .settings-sidebar {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .settings-row {
    align-items: flex-start;
    flex-direction: column;
    gap: var(--iv-space-3);
    padding-top: var(--iv-space-4);
    padding-bottom: var(--iv-space-4);
  }

  .settings-row__control,
  .setting-toggle,
  .background-source {
    justify-content: flex-start;
    width: 100%;
  }

  .background-source {
    flex-wrap: wrap;
  }

  .background-source__filename,
  .background-range {
    width: 100%;
    max-width: none;
  }

}
</style>
