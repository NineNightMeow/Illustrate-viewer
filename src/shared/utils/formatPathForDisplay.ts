const EXTENDED_PATH_PREFIX = "\\\\?\\";
const EXTENDED_UNC_PREFIX = /^\\\\\?\\UNC[\\\\/]/i;
const EXTENDED_LOCAL_PATH_PREFIX = /^\\\\\?\\[A-Za-z]:[\\\\/]/;

export function formatPathForDisplay(path: string) {
  const extendedUnc = path.match(EXTENDED_UNC_PREFIX);
  const normalizedPath = extendedUnc
    ? `\\\\${path.slice(extendedUnc[0].length)}`
    : EXTENDED_LOCAL_PATH_PREFIX.test(path)
      ? path.slice(EXTENDED_PATH_PREFIX.length)
      : path;
  const segments = normalizedPath.split(/[\\/]+/).filter(Boolean);

  return segments.slice(-2).join(" / ") || normalizedPath;
}
