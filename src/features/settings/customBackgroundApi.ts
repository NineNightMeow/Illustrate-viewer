import { invoke } from "@tauri-apps/api/core";

export type CustomBackgroundSelection = {
  path: string;
  name: string;
};

export function chooseCustomBackground() {
  return invoke<CustomBackgroundSelection | null>("choose_custom_background");
}

export function prepareCustomBackground(source: string) {
  return invoke<string>("prepare_custom_background", { source });
}

export function clearCustomBackground() {
  return invoke("clear_custom_background");
}
