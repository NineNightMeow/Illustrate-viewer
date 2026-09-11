import type { AppearanceSettings } from "../../app/appStore";

export function useTheme() {
  function applyAppearance(appearance: AppearanceSettings) {
    if (typeof document === "undefined") return;

    const root = document.documentElement;
    const glassEnabled = appearance.liquidGlass.enabled;
    const intensity = Math.min(100, Math.max(0, appearance.liquidGlass.intensity));
    const glassBlur = glassEnabled ? Math.round(6 + intensity * 0.14) + "px" : "0px";
    const glassSaturation = glassEnabled ? Math.round(104 + intensity * 0.16) + "%" : "100%";
    const glassSurfaceAlpha = glassEnabled ? Math.round(88 - intensity * 0.12) : 100;
    const glassStrongAlpha = glassEnabled ? Math.round(95 - intensity * 0.08) : 100;
    const glassBorderAlpha = glassEnabled ? Math.round(78 + intensity * 0.12) : 100;
    const glassHighlightAlpha = glassEnabled ? Math.round(5 + intensity * 0.05) : 8;
    root.dataset.theme = appearance.theme;
    root.dataset.accent = appearance.accentColor;
    root.dataset.backgroundEnabled = String(appearance.customBackground.enabled);
    root.dataset.liquidGlass = String(glassEnabled);
    root.style.setProperty("--iv-accent", appearance.accentColor);
    root.style.setProperty("--iv-glass-blur", glassBlur);
    root.style.setProperty("--iv-glass-saturation", glassSaturation);
    root.style.setProperty("--iv-glass-surface-alpha", glassSurfaceAlpha + "%");
    root.style.setProperty("--iv-glass-strong-alpha", glassStrongAlpha + "%");
    root.style.setProperty("--iv-glass-border-alpha", glassBorderAlpha + "%");
    root.style.setProperty("--iv-glass-highlight-alpha", glassHighlightAlpha + "%");
    root.style.setProperty("--iv-chrome-surface", glassEnabled ? "var(--iv-glass-bg)" : "var(--iv-bg-primary)");
    root.style.setProperty("--iv-chrome-sidebar-surface", glassEnabled ? "var(--iv-glass-bg)" : "var(--iv-bg-secondary)");
    root.style.setProperty("--iv-content-panel-surface", glassEnabled ? "var(--iv-glass-bg-strong)" : "var(--iv-surface)");
    root.style.setProperty("--iv-floating-surface", glassEnabled ? "var(--iv-glass-bg-strong)" : "var(--iv-surface)");
    root.style.setProperty("--iv-floating-border", glassEnabled ? "var(--iv-glass-border)" : "var(--iv-border-subtle)");
    root.style.setProperty("--iv-background-image-opacity", String(appearance.customBackground.opacity / 100));
    root.style.setProperty("--iv-background-image-blur", `${appearance.customBackground.blur}px`);
    root.style.setProperty("--iv-background-image-dim", String(appearance.customBackground.dim / 100));
    root.style.setProperty("--iv-background-image-fit", appearance.customBackground.fit);
  }

  return { applyAppearance };
}
