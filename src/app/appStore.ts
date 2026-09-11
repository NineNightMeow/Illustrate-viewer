import { defineStore } from "pinia";
import type { AppLocale } from "../i18n";

export type AppTheme = "system" | "light" | "dark";
export type CustomBackgroundFit = "cover" | "contain";
export type CustomBackgroundSourceStatus = "idle" | "available" | "unavailable";

export type AppearanceSettings = {
  theme: AppTheme;
  accentColor: string;
  customBackground: {
    enabled: boolean;
    source: string | null;
    fit: CustomBackgroundFit;
    opacity: number;
    blur: number;
    dim: number;
  };
  liquidGlass: {
    enabled: boolean;
    intensity: number;
  };
};

export type MainWindowGeometry = {
  x: number;
  y: number;
  width: number;
  height: number;
  isMaximized: boolean;
};

export const APP_SETTINGS_STORAGE_KEY = "illustrate-viewer:settings";

export function createDefaultAppearanceSettings(): AppearanceSettings {
  return {
    theme: "system",
    accentColor: "#6B87F2",
    customBackground: {
      enabled: false,
      source: null,
      fit: "cover",
      opacity: 100,
      blur: 0,
      dim: 0,
    },
    liquidGlass: {
      enabled: false,
      intensity: 50,
    },
  };
}

type PersistedSettings = {
  appearance?: Partial<AppearanceSettings>;
  theme?: AppTheme;
  accentColor?: string;
  language?: AppLocale;
  sidebarCollapsed?: boolean;
  liquidGlassEnabled?: boolean;
  mainWindowGeometry?: MainWindowGeometry;
};

function isTheme(value: unknown): value is AppTheme {
  return value === "system" || value === "light" || value === "dark";
}

function isLocale(value: unknown): value is AppLocale {
  return value === "zh-CN" || value === "ja-JP" || value === "en-US";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isAccentColor(value: unknown): value is string {
  return typeof value === "string" && /^#[0-9A-F]{6}$/i.test(value);
}

function isPercentage(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 100;
}

function isBackgroundBlur(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 24;
}

function isCustomBackgroundSource(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0 || value.length > 4_096) return false;

  return /^[a-z]:[\\/]/i.test(value) || value.startsWith("/") || value.startsWith("\\\\");
}

function isBackgroundFit(value: unknown): value is CustomBackgroundFit {
  return value === "cover" || value === "contain";
}

function readAppearanceSettings(saved: PersistedSettings): AppearanceSettings {
  const appearance = createDefaultAppearanceSettings();
  const persisted = isRecord(saved.appearance) ? saved.appearance : {};
  const customBackground = (isRecord(persisted.customBackground)
    ? persisted.customBackground
    : {}) as Partial<AppearanceSettings["customBackground"]>;
  const liquidGlass = (isRecord(persisted.liquidGlass)
    ? persisted.liquidGlass
    : {}) as Partial<AppearanceSettings["liquidGlass"]>;

  if (isTheme(saved.theme)) appearance.theme = saved.theme;
  if (isAccentColor(saved.accentColor)) appearance.accentColor = saved.accentColor;
  if (typeof saved.liquidGlassEnabled === "boolean") appearance.liquidGlass.enabled = saved.liquidGlassEnabled;

  if (isTheme(persisted.theme)) appearance.theme = persisted.theme;
  if (isAccentColor(persisted.accentColor)) appearance.accentColor = persisted.accentColor;
  if (typeof customBackground.enabled === "boolean") appearance.customBackground.enabled = customBackground.enabled;
  if (isCustomBackgroundSource(customBackground.source)) {
    appearance.customBackground.source = customBackground.source;
  }
  if (isBackgroundFit(customBackground.fit)) appearance.customBackground.fit = customBackground.fit;
  if (isPercentage(customBackground.opacity)) appearance.customBackground.opacity = customBackground.opacity;
  if (isBackgroundBlur(customBackground.blur)) appearance.customBackground.blur = customBackground.blur;
  if (isPercentage(customBackground.dim)) appearance.customBackground.dim = customBackground.dim;
  if (typeof liquidGlass.enabled === "boolean") appearance.liquidGlass.enabled = liquidGlass.enabled;
  if (isPercentage(liquidGlass.intensity)) appearance.liquidGlass.intensity = liquidGlass.intensity;

  return appearance;
}

function isMainWindowGeometry(value: unknown): value is MainWindowGeometry {
  if (!value || typeof value !== "object") return false;

  const geometry = value as Partial<MainWindowGeometry>;
  return [geometry.x, geometry.y, geometry.width, geometry.height].every(Number.isFinite)
    && geometry.width! >= 320
    && geometry.height! >= 240
    && geometry.width! <= 8_192
    && geometry.height! <= 8_192
    && Math.abs(geometry.x!) <= 100_000
    && Math.abs(geometry.y!) <= 100_000
    && typeof geometry.isMaximized === "boolean";
}

export const useAppStore = defineStore("app", {
  state: () => ({
    appearance: createDefaultAppearanceSettings(),
    customBackgroundSourceStatus: "idle" as CustomBackgroundSourceStatus,
    language: "en-US" as AppLocale,
    sidebarCollapsed: false,
    mainWindowGeometry: null as MainWindowGeometry | null,
  }),
  actions: {
    hydrate() {
      if (typeof window === "undefined") return;

      try {
        const saved = JSON.parse(window.localStorage.getItem(APP_SETTINGS_STORAGE_KEY) ?? "null") as PersistedSettings | null;
        if (!saved) return;
        this.appearance = readAppearanceSettings(saved);
        this.customBackgroundSourceStatus = "idle";
        if (isLocale(saved.language)) this.language = saved.language;
        if (typeof saved.sidebarCollapsed === "boolean") this.sidebarCollapsed = saved.sidebarCollapsed;
        if (isMainWindowGeometry(saved.mainWindowGeometry)) this.mainWindowGeometry = saved.mainWindowGeometry;
      } catch {
        // Ignore unavailable or malformed local settings and use safe defaults.
      }
    },
    persist() {
      if (typeof window === "undefined") return;

      try {
        window.localStorage.setItem(
          APP_SETTINGS_STORAGE_KEY,
          JSON.stringify({
            appearance: this.appearance,
            language: this.language,
            sidebarCollapsed: this.sidebarCollapsed,
            mainWindowGeometry: this.mainWindowGeometry,
          }),
        );
      } catch {
        // Ignore restricted storage environments.
      }
    },
    setCustomBackgroundSource(source: string | null) {
      this.appearance.customBackground.source = source;
      this.customBackgroundSourceStatus = "idle";
    },
    setCustomBackgroundSourceStatus(status: CustomBackgroundSourceStatus) {
      this.customBackgroundSourceStatus = status;
    },
    clearCustomBackground() {
      this.appearance.customBackground.enabled = false;
      this.appearance.customBackground.source = null;
      this.customBackgroundSourceStatus = "idle";
    },
  },
});
