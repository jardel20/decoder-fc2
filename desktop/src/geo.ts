// Web Mercator helpers (port of gui/src/maps.rs). Numeric GPS validity is not datum validation.
export const MERCATOR_LIMIT = 85.05112878;
export const TILE = 256;
export interface GeoPoint {
  key: string;
  lon: number;
  lat: number;
}
export const renderable = (p: { lat: number; lon: number }) =>
  Number.isFinite(p.lat) &&
  Number.isFinite(p.lon) &&
  Math.abs(p.lat) <= MERCATOR_LIMIT &&
  Math.abs(p.lon) <= 180;
/** Normalised Mercator coordinates, x and y in [0, 1]. */
export const mercatorX = (lon: number) => (lon + 180) / 360;
export const mercatorY = (lat: number) =>
  (1 - Math.asinh(Math.tan((lat * Math.PI) / 180)) / Math.PI) / 2;
export const unmercatorX = (x: number) => x * 360 - 180;
export const unmercatorY = (y: number) =>
  (Math.atan(Math.sinh(Math.PI * (1 - 2 * y))) * 180) / Math.PI;

export interface View {
  lon: number;
  lat: number;
  zoom: number;
}
/** Center and zoom that fit all renderable points, including those crossing the antimeridian. */
export function fitBounds(
  points: { lat: number; lon: number }[],
  width: number,
  height: number,
): View | null {
  const valid = points.filter(renderable);
  if (!valid.length) return null;
  const lons = valid
    .map((p) => ((p.lon % 360) + 360) % 360)
    .sort((a, b) => a - b);
  let largestGap = -1;
  let start = 0;
  for (let i = 0; i < lons.length; i++) {
    const next = i + 1 === lons.length ? lons[0] + 360 : lons[i + 1];
    const gap = next - lons[i];
    if (gap > largestGap) {
      largestGap = gap;
      start = (i + 1) % lons.length;
    }
  }
  const span = 360 - largestGap;
  const lon = ((((lons[start] + span / 2 + 180) % 360) + 360) % 360) - 180;
  let yMin = Infinity,
    yMax = -Infinity;
  for (const p of valid) {
    const y = mercatorY(p.lat);
    yMin = Math.min(yMin, y);
    yMax = Math.max(yMax, y);
  }
  const lat = unmercatorY((yMin + yMax) / 2);
  const horizontal = Math.log2(
    (Math.max(width, 100) - 48) / (TILE * Math.max(span / 360, 1e-8)),
  );
  const vertical = Math.log2(
    (Math.max(height, 100) - 48) / (TILE * Math.max(yMax - yMin, 1e-8)),
  );
  return {
    lon,
    lat,
    zoom: Math.min(17, Math.max(0, Math.min(horizontal, vertical))),
  };
}

/** One plotted record; `key` is work-group-record in source order, never the stored ID. */
export interface MapPoint {
  key: string;
  work: number;
  workName: string;
  workNumber: number;
  groupIndex: number;
  group: number;
  record: number;
  id: number;
  point: number;
  date: string | null;
  time: string | null;
  a: string;
  b: string;
  total: string;
  lat: number;
  lon: number;
  warnings: string[];
}
