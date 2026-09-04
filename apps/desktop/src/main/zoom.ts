import type { DesktopZoomAction } from '@typie/lib/desktop';

// Electron scales zoom as 1.2 ** level, so these integer bounds approximate 50%–300%.
const MIN_ZOOM_LEVEL = -4;
const MAX_ZOOM_LEVEL = 6;

export const normalizeZoomLevel = (value: unknown): number => {
  if (!Number.isSafeInteger(value)) return 0;
  const level = value as number;
  return level >= MIN_ZOOM_LEVEL && level <= MAX_ZOOM_LEVEL ? level : 0;
};

export const nextZoomLevel = (current: number, action: DesktopZoomAction): number => {
  if (action === 'reset') return 0;
  const next = current + (action === 'in' ? 1 : -1);
  return Math.max(MIN_ZOOM_LEVEL, Math.min(MAX_ZOOM_LEVEL, next));
};
