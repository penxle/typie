import { clamp } from '@typie/ui/utils';
import { CURSOR_VISIBLE_MARGIN, TYPEWRITER_MIN_BOTTOM_PADDING } from './constants';

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

export function resolveKeepVisibleBottomPadding({
  visibleArea,
  margin = CURSOR_VISIBLE_MARGIN,
  minPadding = 0,
}: {
  visibleArea?: EditorVisibleArea;
  margin?: number;
  minPadding?: number;
}): number {
  const area = normalizeVisibleArea(visibleArea);
  const requiredPadding = area.bottomInset + Math.max(0, finiteOrZero(margin));
  return Math.max(Math.max(0, finiteOrZero(minPadding)), requiredPadding);
}

export function resolveTypewriterBottomPadding({
  clientHeight,
  targetHeight,
  visibleArea,
  position,
  trailingBottomMargin = 0,
  minPadding = TYPEWRITER_MIN_BOTTOM_PADDING,
}: {
  clientHeight: number;
  targetHeight: number;
  visibleArea?: EditorVisibleArea;
  position: number;
  trailingBottomMargin?: number;
  minPadding?: number;
}): number {
  const area = normalizeVisibleArea(visibleArea);
  const safeTargetHeight = Math.max(0, finiteOrZero(targetHeight));
  const usableHeight = Math.max(0, finiteOrZero(clientHeight) - area.topInset - area.bottomInset);
  const availableRange = Math.max(0, usableHeight - safeTargetHeight);
  const clampedPosition = clamp(finiteOrZero(position), 0, 1);
  const spaceNeededBelowTargetTop = area.bottomInset + (1 - clampedPosition) * availableRange + safeTargetHeight;
  const intrinsicSpaceBelowTargetTop = Math.max(0, finiteOrZero(trailingBottomMargin)) + safeTargetHeight;
  const requiredPadding = spaceNeededBelowTargetTop - intrinsicSpaceBelowTargetTop;

  return Math.max(Math.max(0, finiteOrZero(minPadding)), requiredPadding);
}
