import { invoke } from "@tauri-apps/api/core";
import type { ImageAsset } from "../library/types";

type OpenReferenceWindowResult = {
  referenceId: string;
  windowLabel: string;
  wasExisting: boolean;
};

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function openReferenceWindow(asset: ImageAsset) {
  if (!isTauriEnvironment) return Promise.resolve<OpenReferenceWindowResult | null>(null);

  return invoke<OpenReferenceWindowResult>("open_reference_window", {
    asset: {
      imageId: asset.id,
      libraryId: asset.libraryId,
      sourcePath: asset.path,
    },
  });
}
