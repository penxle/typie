import { clamp } from '@typie/ui/utils';
import { CONTINUOUS_VIEW_PADDING } from './constants';
import { elasticDisplayZoom } from './zoom-motion';

export const MIN_DOCUMENT_DISPLAY_WIDTH = 100;
export const MAX_DOCUMENT_ZOOM = 2;
export const FIT_WIDTH_ZOOM_SNAP_THRESHOLD = 0.02;
export const UNIT_ZOOM_SNAP_THRESHOLD = 0.02;
export const RENDER_ZOOM_DEBOUNCE_MS = 120;
export const RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS = 160;
export const RENDER_ZOOM_MAX_COMMIT_DELAY_MS = 300;
export const RENDER_ZOOM_SCALE_RATIO_THRESHOLD = 1.18;
export const ZOOM_EPSILON = 0.0001;
const DISCRETE_ZOOM_STEP = 0.1;

export type ZoomBounds = {
  min: number;
  max: number;
};

export type DocumentZoomLayout = { type: 'continuous'; maxWidth: number } | { type: 'paginated'; pageWidth: number };

export type DocumentZoomLandmark = 'minimum' | 'fit-width' | 'unit' | 'maximum';

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

export function resolveDocumentZoomStepTarget({
  zoom,
  direction,
  layout,
  viewportWidth,
}: {
  zoom: number;
  direction: -1 | 1;
  layout: DocumentZoomLayout;
  viewportWidth: number;
}): number | null {
  const bounds = computeDocumentZoomBounds(layout);
  const current = clampDocumentZoom(zoom, bounds);
  const candidates = [bounds.min, computeDocumentFitWidthZoom(layout, viewportWidth), clampDocumentZoom(1, bounds), bounds.max];
  const firstGridIndex = Math.ceil((bounds.min - ZOOM_EPSILON) / DISCRETE_ZOOM_STEP);
  const lastGridIndex = Math.floor((bounds.max + ZOOM_EPSILON) / DISCRETE_ZOOM_STEP);
  for (let index = firstGridIndex; index <= lastGridIndex; index += 1) {
    candidates.push(index * DISCRETE_ZOOM_STEP);
  }

  const ordered = [...new Set(candidates.map((candidate) => clampDocumentZoom(candidate, bounds)))].toSorted((a, b) => a - b);
  if (direction > 0) return ordered.find((candidate) => candidate > current + ZOOM_EPSILON) ?? null;
  return ordered.findLast((candidate) => candidate < current - ZOOM_EPSILON) ?? null;
}

export function resolveDirectDocumentZoom(zoom: number, layout: DocumentZoomLayout): number {
  const bounds = computeDocumentZoomBounds(layout);
  return elasticDisplayZoom(zoom, bounds) ?? bounds.min;
}

export function resolveDocumentZoomIndicator(displayZoom: number, layout: DocumentZoomLayout): number {
  return clampDocumentZoom(displayZoom, computeDocumentZoomBounds(layout));
}

export function resolveDocumentZoomLandmark({
  zoom,
  layout,
  viewportWidth,
}: {
  zoom: number;
  layout: DocumentZoomLayout;
  viewportWidth: number;
}): DocumentZoomLandmark | null {
  const layoutWidth = layout.type === 'continuous' ? layout.maxWidth : layout.pageWidth;
  if (!Number.isFinite(zoom) || zoom <= 0 || !Number.isFinite(layoutWidth) || layoutWidth <= 0) return null;
  if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) return null;

  const bounds = computeDocumentZoomBounds(layout);
  const unitZoom = clampDocumentZoom(1, bounds);
  if (zoomEquals(zoom, unitZoom)) return 'unit';

  const naturalFitWidthZoom = viewportWidth / documentZoomWidth(layout);
  if (naturalFitWidthZoom >= bounds.min && naturalFitWidthZoom <= bounds.max && zoomEquals(zoom, naturalFitWidthZoom)) return 'fit-width';
  if (zoomEquals(zoom, bounds.min)) return 'minimum';
  if (zoomEquals(zoom, bounds.max)) return 'maximum';
  return null;
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
