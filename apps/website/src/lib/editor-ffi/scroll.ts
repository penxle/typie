import { clamp } from '@typie/ui/utils';
import { CURSOR_VISIBLE_MARGIN, TYPEWRITER_MIN_BOTTOM_PADDING } from './constants';
import type { Size } from '@typie/editor-ffi/browser';

export type EditorVisibleArea = {
  topInset: number;
  bottomInset: number;
};

export type RevealTargetSpan = {
  targetTop: number;
  targetBottom: number;
};

export type ScrollContainerMetrics = {
  scrollTop: number;
  clientHeight: number;
  scrollHeight: number;
};

const DEFAULT_VISIBLE_AREA: EditorVisibleArea = {
  topInset: 0,
  bottomInset: 0,
};

function finiteOrZero(value: number): number {
  return Number.isFinite(value) ? value : 0;
}

function resolveMaxScrollTop(metrics: Pick<ScrollContainerMetrics, 'clientHeight' | 'scrollHeight'>): number {
  return Math.max(0, finiteOrZero(metrics.scrollHeight) - finiteOrZero(metrics.clientHeight));
}

function normalizeVisibleArea(visibleArea: EditorVisibleArea | undefined): EditorVisibleArea {
  return {
    topInset: Math.max(0, finiteOrZero(visibleArea?.topInset ?? DEFAULT_VISIBLE_AREA.topInset)),
    bottomInset: Math.max(0, finiteOrZero(visibleArea?.bottomInset ?? DEFAULT_VISIBLE_AREA.bottomInset)),
  };
}

function nearlySameScroll(a: number, b: number): boolean {
  return Math.abs(a - b) <= 1;
}

export function resolveNearestScrollTop({
  scrollTop,
  clientHeight,
  scrollHeight,
  targetTop,
  targetBottom,
  visibleArea,
  margin = CURSOR_VISIBLE_MARGIN,
}: ScrollContainerMetrics &
  RevealTargetSpan & {
    visibleArea?: EditorVisibleArea;
    margin?: number;
  }): number | null {
  const area = normalizeVisibleArea(visibleArea);
  const safeMargin = Math.max(0, finiteOrZero(margin));
  const safeScrollTop = finiteOrZero(scrollTop);
  const safeClientHeight = Math.max(0, finiteOrZero(clientHeight));
  const rangeTop = area.topInset + safeMargin;
  const rangeBottom = safeClientHeight - area.bottomInset - safeMargin;
  if (rangeBottom <= rangeTop) {
    return null;
  }

  let nextTop: number | null = null;
  if (targetBottom - safeScrollTop > rangeBottom) {
    nextTop = finiteOrZero(targetBottom) - rangeBottom;
  } else if (targetTop - safeScrollTop < rangeTop) {
    nextTop = finiteOrZero(targetTop) - rangeTop;
  }

  if (nextTop === null) {
    return null;
  }

  const clamped = clamp(nextTop, 0, resolveMaxScrollTop({ clientHeight, scrollHeight }));
  return nearlySameScroll(clamped, safeScrollTop) ? null : clamped;
}

export function resolveTypewriterScrollTop({
  scrollTop,
  clientHeight,
  scrollHeight,
  targetTop,
  targetBottom,
  visibleArea,
  position,
}: ScrollContainerMetrics &
  RevealTargetSpan & {
    visibleArea?: EditorVisibleArea;
    position: number;
  }): number | null {
  const area = normalizeVisibleArea(visibleArea);
  const safeClientHeight = Math.max(0, finiteOrZero(clientHeight));
  const usableHeight = Math.max(0, safeClientHeight - area.topInset - area.bottomInset);
  if (usableHeight <= 0) {
    return null;
  }

  const targetHeight = Math.max(0, finiteOrZero(targetBottom) - finiteOrZero(targetTop));
  const clampedPosition = clamp(finiteOrZero(position), 0, 1);
  const targetTopInViewport = area.topInset + Math.max(0, usableHeight - targetHeight) * clampedPosition;
  const clamped = clamp(finiteOrZero(targetTop) - targetTopInViewport, 0, resolveMaxScrollTop({ clientHeight, scrollHeight }));
  const safeScrollTop = finiteOrZero(scrollTop);
  return nearlySameScroll(clamped, safeScrollTop) ? null : clamped;
}

export function resolveDistanceToPagesBottom({
  pageSizes,
  pageIdx,
  targetY,
  displayZoom = 1,
  pageGap = 0,
}: {
  pageSizes: readonly Size[];
  pageIdx: number;
  targetY: number;
  displayZoom?: number;
  pageGap?: number;
}): number | null {
  if (!Number.isInteger(pageIdx) || pageIdx < 0 || pageIdx >= pageSizes.length) {
    return null;
  }

  const zoom = finiteOrZero(displayZoom) > 0 ? finiteOrZero(displayZoom) : 1;
  const gap = Math.max(0, finiteOrZero(pageGap));
  const page = pageSizes[pageIdx];
  const currentPageDistance = Math.max(0, finiteOrZero(page.height) - Math.max(0, finiteOrZero(targetY)));
  let distance = currentPageDistance;

  for (let i = pageIdx + 1; i < pageSizes.length; i++) {
    distance += gap + Math.max(0, finiteOrZero(pageSizes[i].height));
  }

  return distance * zoom;
}

export function resolveKeepVisibleBottomPadding({
  distanceToContentBottom,
  visibleArea,
  margin = CURSOR_VISIBLE_MARGIN,
  minPadding = 0,
}: {
  distanceToContentBottom: number;
  visibleArea?: EditorVisibleArea;
  margin?: number;
  minPadding?: number;
}): number {
  const area = normalizeVisibleArea(visibleArea);
  const safeDistanceToContentBottom = Math.max(0, finiteOrZero(distanceToContentBottom));
  const requiredPadding = area.bottomInset + Math.max(0, finiteOrZero(margin)) - safeDistanceToContentBottom;
  return Math.max(Math.max(0, finiteOrZero(minPadding)), requiredPadding);
}

export function resolveTypewriterBottomPadding({
  clientHeight,
  targetHeight,
  distanceToContentBottom,
  visibleArea,
  position,
  minPadding = TYPEWRITER_MIN_BOTTOM_PADDING,
}: {
  clientHeight: number;
  targetHeight: number;
  distanceToContentBottom: number;
  visibleArea?: EditorVisibleArea;
  position: number;
  minPadding?: number;
}): number {
  const area = normalizeVisibleArea(visibleArea);
  const safeTargetHeight = Math.max(0, finiteOrZero(targetHeight));
  const usableHeight = Math.max(0, finiteOrZero(clientHeight) - area.topInset - area.bottomInset);
  const availableRange = Math.max(0, usableHeight - safeTargetHeight);
  const clampedPosition = clamp(finiteOrZero(position), 0, 1);
  const spaceNeededBelowTargetTop = area.bottomInset + (1 - clampedPosition) * availableRange + safeTargetHeight;
  const requiredPadding = spaceNeededBelowTargetTop - Math.max(0, finiteOrZero(distanceToContentBottom));

  return Math.max(Math.max(0, finiteOrZero(minPadding)), requiredPadding);
}
