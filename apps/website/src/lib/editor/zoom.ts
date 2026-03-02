import { clamp } from '@typie/ui/utils';

export const MIN_DOCUMENT_DISPLAY_WIDTH = 100;
export const MAX_DOCUMENT_ZOOM = 2;
export const FIT_WIDTH_ZOOM_SNAP_THRESHOLD = 0.02;
export const UNIT_ZOOM_SNAP_THRESHOLD = 0.02;
export const RENDER_ZOOM_DEBOUNCE_MS = 120;
export const ZOOM_EPSILON = 0.0001;

export type ZoomBounds = {
  min: number;
  max: number;
};

export function computePaginatedZoomBounds(pageWidth: number, minDisplayWidth = MIN_DOCUMENT_DISPLAY_WIDTH): ZoomBounds {
  const safePageWidth = Number.isFinite(pageWidth) && pageWidth > 0 ? pageWidth : 1;
  const minZoom = clamp(minDisplayWidth / safePageWidth, 0.01, Number.POSITIVE_INFINITY);
  const maxZoom = clamp(MAX_DOCUMENT_ZOOM, minZoom, Number.POSITIVE_INFINITY);
  return { min: minZoom, max: maxZoom };
}

export function clampDocumentZoom(zoom: number, bounds: ZoomBounds): number {
  if (!Number.isFinite(zoom)) {
    return bounds.min;
  }
  return clamp(zoom, bounds.min, bounds.max);
}

export function computePaginatedFitWidthZoom(pageWidth: number, viewportWidth: number): number {
  const bounds = computePaginatedZoomBounds(pageWidth);
  const safePageWidth = Number.isFinite(pageWidth) && pageWidth > 0 ? pageWidth : 1;
  const safeViewportWidth = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : safePageWidth;
  return clamp(safeViewportWidth / safePageWidth, bounds.min, bounds.max);
}

export function computeInitialPaginatedZoom(pageWidth: number, viewportWidth: number): number {
  return Math.min(computePaginatedFitWidthZoom(pageWidth, viewportWidth), 1);
}

export function clampPaginatedZoom({ zoom, pageWidth, viewportWidth }: { zoom: number; pageWidth: number; viewportWidth: number }): number {
  const bounds = computePaginatedZoomBounds(pageWidth);
  const clamped = clampDocumentZoom(zoom, bounds);
  const fitWidthZoom = computePaginatedFitWidthZoom(pageWidth, viewportWidth);
  const unitZoom = clampDocumentZoom(1, bounds);

  let snapped: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;

  const fitWidthDistance = Math.abs(clamped - fitWidthZoom);
  if (fitWidthDistance <= FIT_WIDTH_ZOOM_SNAP_THRESHOLD) {
    snapped = fitWidthZoom;
    bestDistance = fitWidthDistance;
  }

  const unitDistance = Math.abs(clamped - unitZoom);
  if (unitDistance <= UNIT_ZOOM_SNAP_THRESHOLD && unitDistance < bestDistance) {
    snapped = unitZoom;
  }

  return snapped ?? clamped;
}

export function renderZoomForDisplay(displayZoom: number): number {
  if (!Number.isFinite(displayZoom)) {
    return 1;
  }
  return displayZoom <= 0 ? 0.01 : displayZoom;
}

export function zoomEquals(a: number, b: number): boolean {
  return Math.abs(a - b) < ZOOM_EPSILON;
}

export function zoomDiffers(a: number, b: number): boolean {
  return !zoomEquals(a, b);
}
