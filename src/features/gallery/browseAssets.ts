import type { ImageAsset } from "../library/types";

export type GallerySort = "name" | "createdAt" | "modifiedAt";
export type GalleryFileType = "all" | ImageAsset["extension"];

export function deriveGalleryAssets(
  assets: ImageAsset[],
  sort: GallerySort,
  fileType: GalleryFileType,
  locale: string,
) {
  const collator = new Intl.Collator(locale, { numeric: true, sensitivity: "base" });
  const matching = fileType === "all"
    ? assets.slice()
    : assets.filter((asset) => asset.extension === fileType);
  const compareName = (left: ImageAsset, right: ImageAsset) =>
    collator.compare(left.filename, right.filename) || collator.compare(left.id, right.id);

  return matching.sort((left, right) => {
    if (sort === "name") return compareName(left, right);

    const leftTime = sort === "createdAt" ? left.createdAt : left.modifiedAt;
    const rightTime = sort === "createdAt" ? right.createdAt : right.modifiedAt;
    if (leftTime === null) return rightTime === null ? compareName(left, right) : 1;
    if (rightTime === null) return -1;
    return rightTime - leftTime || compareName(left, right);
  });
}
