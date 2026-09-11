import { invoke } from "@tauri-apps/api/core";

export type Color = {
  hex: string;
  rgb: string;
  hsl: string;
  percentage: number;
};

export type ImageColorMetadata = {
  imageId: string;
  dominantColors: Color[];
  colorCount: number;
  generatedAt: number;
  sourceModifiedAt: number;
};

export type ColorAnalysisError = {
  code: string;
  message: string;
};

export function getImageColorMetadata(imageId: string) {
  return invoke<ImageColorMetadata | null>("get_image_color_metadata", { imageId });
}

export function analyzeImageColors(imageId: string) {
  return invoke<ImageColorMetadata>("analyze_image_colors", { imageId });
}

export function sampleImageColor(imageId: string, x: number, y: number) {
  return invoke<Color>("sample_image_color", { imageId, x, y });
}
