// Pure seam-sampling geometry for the surface-probe. Multi-tile GL pages blit each
// retained tile to the backbuffer independently, so a wrong dst-Y or tile-placement
// bug shows up as a discontinuity across a tile boundary. Given the tile y0 list
// (from `debug_surface_tile_ranges`) and the device size, this produces the top-left
// corners of BLOCK×BLOCK sample windows straddling every interior boundary. No DOM /
// editor / localStorage access, so it is unit-testable in isolation.

export const SEAM_BLOCK = 2;

// Sample rows just above and at each interior boundary (`y0 - block` and `y0`), across
// a few evenly spread columns. Points are clamped into `[0, size - block]`, deduped,
// and returned in row-major order. An empty / single-tile page yields no seam points.
export function seamSamplePoints(
  tileY0s: readonly number[],
  width: number,
  height: number,
  block: number = SEAM_BLOCK,
): [number, number][] {
  if (width < block || height < block) return [];

  const clamp = (v: number, max: number) => Math.max(0, Math.min(v, max));
  const maxX = width - block;
  const maxY = height - block;

  const columns = [clamp(8, maxX), clamp(Math.floor(width / 2) - Math.floor(block / 2), maxX), clamp(width - 8 - block, maxX)];

  const points: [number, number][] = [];
  const seen = new Set<string>();
  const push = (x: number, y: number) => {
    const key = `${x},${y}`;
    if (seen.has(key)) return;
    seen.add(key);
    points.push([x, y]);
  };

  for (const y0 of tileY0s) {
    // Only interior boundaries — the page top (y0 <= 0) and any boundary at/below the
    // bottom edge are not seams between two tiles.
    if (y0 <= 0 || y0 >= height) continue;
    for (const row of [y0 - block, y0]) {
      const y = clamp(row, maxY);
      for (const x of columns) push(x, y);
    }
  }

  return points;
}
