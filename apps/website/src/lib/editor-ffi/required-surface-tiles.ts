import type { Rect } from '@typie/editor-ffi/browser';

const TILE_SIZE = 512;
const MAX_TILES = 128;

// Logical page regions become a stable device-pixel grid shared with the Rust rasterizer.
export function requiredSurfaceTiles(width: number, height: number, scale: number, regions: readonly Rect[]): number[] {
  const pixelWidth = Math.round(width * scale);
  const pixelHeight = Math.round(height * scale);
  if (!Number.isFinite(pixelWidth) || !Number.isFinite(pixelHeight) || !Number.isFinite(scale) || scale <= 0) return [];
  const tiles = new Map<string, number[]>();
  for (const region of regions) {
    if (![region.x, region.y, region.width, region.height].every(Number.isFinite)) continue;
    const left = Math.max(0, Math.min(pixelWidth, Math.floor(region.x * scale)));
    const top = Math.max(0, Math.min(pixelHeight, Math.floor(region.y * scale)));
    const right = Math.max(0, Math.min(pixelWidth, Math.ceil((region.x + region.width) * scale)));
    const bottom = Math.max(0, Math.min(pixelHeight, Math.ceil((region.y + region.height) * scale)));
    if (right <= left || bottom <= top) continue;
    const firstColumn = Math.floor(left / TILE_SIZE);
    const lastColumn = Math.floor((right - 1) / TILE_SIZE);
    const firstRow = Math.floor(top / TILE_SIZE);
    const lastRow = Math.floor((bottom - 1) / TILE_SIZE);
    if ((lastColumn - firstColumn + 1) * (lastRow - firstRow + 1) > MAX_TILES) throw new Error('Visible surface exceeds tile budget');
    for (let row = firstRow; row <= lastRow; row++) {
      for (let column = firstColumn; column <= lastColumn; column++) {
        const x = column * TILE_SIZE;
        const y = row * TILE_SIZE;
        tiles.set(`${column},${row}`, [x, y, Math.min(x + TILE_SIZE, pixelWidth), Math.min(y + TILE_SIZE, pixelHeight)]);
        if (tiles.size > MAX_TILES) throw new Error('Visible surface exceeds tile budget');
      }
    }
  }
  return [...tiles.values()].toSorted((a, b) => a[1] - b[1] || a[0] - b[0]).flat();
}
