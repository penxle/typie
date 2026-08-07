import { clamp } from '@typie/ui/utils';
import { CURSOR_VISIBLE_MARGIN, TYPEWRITER_MIN_BOTTOM_PADDING } from './constants';
import type { VerticalSpan } from './required-surface-pages';

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

function uniqueClampedScrollTops(scrollTops: readonly number[], maximumScrollTop: number): number[] {
  const result: number[] = [];
  for (const scrollTop of scrollTops) {
    const clamped = clamp(finiteOrZero(scrollTop), 0, maximumScrollTop);
    if (result.every((existing) => !nearlySameScroll(existing, clamped))) result.push(clamped);
  }
  return result;
}

export function resolveGuardedScrollTop({
  scrollTop,
  clientHeight,
  scrollHeight,
  targetTop,
  targetBottom,
  visibleArea,
  margin = CURSOR_VISIBLE_MARGIN,
  oversizedAlignment = 'cursor_guard',
  oversizedMinimumVisibleHeight,
}: ScrollContainerMetrics &
  RevealTargetSpan & {
    visibleArea?: EditorVisibleArea;
    margin?: number;
    oversizedAlignment?: 'cursor_guard' | 'start';
    oversizedMinimumVisibleHeight?: number;
  }): number | null {
  const area = normalizeVisibleArea(visibleArea);
  const safeMargin = Math.max(0, finiteOrZero(margin));
  const safeScrollTop = finiteOrZero(scrollTop);
  const safeClientHeight = Math.max(0, finiteOrZero(clientHeight));
  const safeTargetTop = finiteOrZero(targetTop);
  const safeTargetBottom = finiteOrZero(targetBottom);
  const rangeTop = area.topInset + safeMargin;
  const rangeBottom = safeClientHeight - area.bottomInset - safeMargin;
  if (rangeBottom <= rangeTop) {
    const visibleTop = area.topInset;
    const visibleBottom = safeClientHeight - area.bottomInset;
    if (visibleBottom <= visibleTop) return null;

    const targetTopInViewport = safeTargetTop - safeScrollTop;
    const targetBottomInViewport = safeTargetBottom - safeScrollTop;
    if (targetTopInViewport >= visibleTop && targetBottomInViewport <= visibleBottom) return null;

    const targetCenter = safeTargetTop + (safeTargetBottom - safeTargetTop) / 2;
    const visibleCenter = visibleTop + (visibleBottom - visibleTop) / 2;
    const centered = clamp(targetCenter - visibleCenter, 0, resolveMaxScrollTop({ clientHeight, scrollHeight }));
    return nearlySameScroll(centered, safeScrollTop) ? null : centered;
  }

  let nextTop: number | null = null;
  const targetHeight = Math.max(0, safeTargetBottom - safeTargetTop);
  const rangeHeight = rangeBottom - rangeTop;
  if (targetHeight > rangeHeight) {
    if (oversizedAlignment === 'start') {
      nextTop = safeTargetTop - rangeTop;
    } else if (oversizedMinimumVisibleHeight === undefined) {
      if (safeTargetTop - safeScrollTop <= rangeTop && safeTargetBottom - safeScrollTop >= rangeBottom) {
        return null;
      }
      if (safeTargetBottom - safeScrollTop > rangeBottom) {
        nextTop = safeTargetBottom - rangeBottom;
      } else if (safeTargetTop - safeScrollTop < rangeTop) {
        nextTop = safeTargetTop - rangeTop;
      }
    } else {
      const targetTopInViewport = safeTargetTop - safeScrollTop;
      const targetBottomInViewport = safeTargetBottom - safeScrollTop;
      const minimumVisibleHeight = Math.min(rangeHeight, Math.max(0, finiteOrZero(oversizedMinimumVisibleHeight)));
      const visibleHeight = Math.max(0, Math.min(targetBottomInViewport, rangeBottom) - Math.max(targetTopInViewport, rangeTop));
      if (visibleHeight >= minimumVisibleHeight) return null;

      if (targetTopInViewport > rangeTop) {
        nextTop = safeTargetTop - (rangeBottom - minimumVisibleHeight);
      } else if (targetBottomInViewport < rangeBottom) {
        nextTop = safeTargetBottom - (rangeTop + minimumVisibleHeight);
      }
    }
  } else if (safeTargetBottom - safeScrollTop > rangeBottom) {
    nextTop = safeTargetBottom - rangeBottom;
  } else if (safeTargetTop - safeScrollTop < rangeTop) {
    nextTop = safeTargetTop - rangeTop;
  }

  if (nextTop === null) {
    return null;
  }

  const clamped = clamp(nextTop, 0, resolveMaxScrollTop({ clientHeight, scrollHeight }));
  return nearlySameScroll(clamped, safeScrollTop) ? null : clamped;
}

export function resolveInstantRevealPreparationViewports({
  mode,
  scrollTop,
  clientHeight,
  scrollHeight,
  targetTop,
  targetBottom,
  visibleArea,
  margin = CURSOR_VISIBLE_MARGIN,
  position = 0.5,
  oversizedMinimumVisibleHeight,
}: ScrollContainerMetrics &
  RevealTargetSpan & {
    mode: 'cursor_guard' | 'typewriter';
    visibleArea?: EditorVisibleArea;
    margin?: number;
    position?: number;
    oversizedMinimumVisibleHeight?: number;
  }): VerticalSpan[] {
  const safeClientHeight = Math.max(0, finiteOrZero(clientHeight));
  if (safeClientHeight <= 0) return [];

  const area = normalizeVisibleArea(visibleArea);
  const safeScrollTop = finiteOrZero(scrollTop);
  const safeTargetTop = finiteOrZero(targetTop);
  const safeTargetBottom = finiteOrZero(targetBottom);
  const maximumScrollTop = resolveMaxScrollTop({ clientHeight, scrollHeight });
  let destinations: number[];

  const safeMargin = Math.max(0, finiteOrZero(margin));
  const rangeTop = area.topInset + safeMargin;
  const rangeBottom = safeClientHeight - area.bottomInset - safeMargin;
  const targetHeight = Math.max(0, safeTargetBottom - safeTargetTop);
  const useTypewriter = mode === 'typewriter' && targetHeight <= Math.max(0, rangeBottom - rangeTop);

  if (useTypewriter) {
    const usableHeight = Math.max(0, safeClientHeight - area.topInset - area.bottomInset);
    if (usableHeight <= 0) return [];
    destinations = [
      resolveTypewriterScrollTop({
        scrollTop: safeScrollTop,
        clientHeight: safeClientHeight,
        scrollHeight,
        targetTop: safeTargetTop,
        targetBottom: safeTargetBottom,
        visibleArea: area,
        position,
      }) ?? safeScrollTop,
    ];
  } else {
    if (rangeBottom <= rangeTop) {
      const visibleTop = area.topInset;
      const visibleBottom = safeClientHeight - area.bottomInset;
      if (visibleBottom <= visibleTop) return [];
      const targetTopInViewport = safeTargetTop - safeScrollTop;
      const targetBottomInViewport = safeTargetBottom - safeScrollTop;
      destinations =
        targetTopInViewport >= visibleTop && targetBottomInViewport <= visibleBottom
          ? [safeScrollTop]
          : [safeTargetTop + (safeTargetBottom - safeTargetTop) / 2 - (visibleTop + (visibleBottom - visibleTop) / 2)];
    } else {
      const targetHeight = Math.max(0, safeTargetBottom - safeTargetTop);
      const targetTopInViewport = safeTargetTop - safeScrollTop;
      const targetBottomInViewport = safeTargetBottom - safeScrollTop;
      destinations = [];
      if (targetHeight > rangeBottom - rangeTop) {
        if (oversizedMinimumVisibleHeight === undefined) {
          destinations.push(safeScrollTop, safeTargetTop - rangeTop, safeTargetBottom - rangeBottom);
        } else {
          const minimumVisibleHeight = Math.min(rangeBottom - rangeTop, Math.max(0, finiteOrZero(oversizedMinimumVisibleHeight)));
          destinations.push(
            safeScrollTop,
            safeTargetTop - (rangeBottom - minimumVisibleHeight),
            safeTargetBottom - (rangeTop + minimumVisibleHeight),
          );
        }
      } else {
        if (targetTopInViewport >= rangeTop && targetBottomInViewport <= rangeBottom) destinations.push(safeScrollTop);
        destinations.push(safeTargetTop - rangeTop, safeTargetBottom - rangeBottom);
      }
    }
  }

  return uniqueClampedScrollTops(destinations, maximumScrollTop).map((top) => ({ top, bottom: top + safeClientHeight }));
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
  const guardedHeight = Math.max(0, usableHeight - CURSOR_VISIBLE_MARGIN * 2);
  if (targetHeight > guardedHeight) {
    return resolveGuardedScrollTop({
      scrollTop,
      clientHeight,
      scrollHeight,
      targetTop,
      targetBottom,
      visibleArea: area,
    });
  }
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
  return Math.max(0, finiteOrZero(minPadding), requiredPadding);
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

  return Math.max(0, finiteOrZero(minPadding), requiredPadding);
}
