import { clamp } from '@typie/ui/utils';
import { CONTINUOUS_VIEW_PADDING } from './constants';

export const MIN_DOCUMENT_DISPLAY_WIDTH = 100;
export const MAX_DOCUMENT_ZOOM = 2;
export const FIT_WIDTH_ZOOM_SNAP_THRESHOLD = 0.02;
export const UNIT_ZOOM_SNAP_THRESHOLD = 0.02;
export const RENDER_ZOOM_DEBOUNCE_MS = 120;
export const RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS = 160;
export const RENDER_ZOOM_MAX_COMMIT_DELAY_MS = 300;
export const RENDER_ZOOM_SCALE_RATIO_THRESHOLD = 1.18;
export const ZOOM_EPSILON = 0.0001;

export type ZoomBounds = {
  min: number;
  max: number;
};

export type DocumentZoomLayout = { type: 'continuous'; maxWidth: number } | { type: 'paginated'; pageWidth: number };

export function documentZoomWidth(layout: DocumentZoomLayout): number {
  const width = layout.type === 'continuous' ? layout.maxWidth + CONTINUOUS_VIEW_PADDING * 2 : layout.pageWidth;
  return Number.isFinite(width) && width > 0 ? width : 1;
}

export function computeDocumentZoomBounds(layout: DocumentZoomLayout, minDisplayWidth = MIN_DOCUMENT_DISPLAY_WIDTH): ZoomBounds {
  const minZoom = clamp(minDisplayWidth / documentZoomWidth(layout), 0.01, Infinity);
  return { min: minZoom, max: clamp(MAX_DOCUMENT_ZOOM, minZoom, Infinity) };
}

export function clampDocumentZoom(zoom: number, bounds: ZoomBounds): number {
  if (!Number.isFinite(zoom)) {
    return bounds.min;
  }
  return clamp(zoom, bounds.min, bounds.max);
}

export function computeDocumentFitWidthZoom(layout: DocumentZoomLayout, viewportWidth: number): number {
  const width = documentZoomWidth(layout);
  const bounds = computeDocumentZoomBounds(layout);
  const safeViewportWidth = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : width;
  return clamp(safeViewportWidth / width, bounds.min, bounds.max);
}

export function computeInitialDocumentZoom(layout: DocumentZoomLayout, viewportWidth: number): number {
  return layout.type === 'continuous'
    ? clampDocumentZoom(1, computeDocumentZoomBounds(layout))
    : Math.min(computeDocumentFitWidthZoom(layout, viewportWidth), 1);
}

export function clampDocumentLayoutZoom({
  zoom,
  layout,
  viewportWidth,
}: {
  zoom: number;
  layout: DocumentZoomLayout;
  viewportWidth: number;
}): number {
  const bounds = computeDocumentZoomBounds(layout);
  const clamped = clampDocumentZoom(zoom, bounds);
  const fitWidthZoom = computeDocumentFitWidthZoom(layout, viewportWidth);
  const unitZoom = clampDocumentZoom(1, bounds);

  let snapped: number | null = null;
  let bestDistance = Infinity;

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

export function resolveContinuousLayoutViewportWidth({
  viewportWidth,
  committedZoom,
}: {
  viewportWidth: number;
  committedZoom: number;
}): number {
  const safeWidth = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : 1;
  const safeZoom = renderZoomForDisplay(committedZoom);
  return safeWidth / Math.min(safeZoom, 1);
}

export function resolveContinuousViewPadding(displayZoom: number): number {
  return CONTINUOUS_VIEW_PADDING * renderZoomForDisplay(displayZoom);
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
