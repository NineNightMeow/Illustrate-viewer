export type ViewerEscapeLayer = "colorPicker" | "informationPanel" | "backgroundMenu" | "viewer";

export function getViewerEscapeLayer(state: {
  colorPickerOpen: boolean;
  informationPanelOpen: boolean;
  backgroundMenuOpen: boolean;
}): ViewerEscapeLayer {
  if (state.colorPickerOpen) return "colorPicker";
  if (state.informationPanelOpen) return "informationPanel";
  if (state.backgroundMenuOpen) return "backgroundMenu";
  return "viewer";
}
