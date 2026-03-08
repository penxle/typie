import type { ScrollViewport } from './scroll-viewport';

export type DragScrollAxis = 'vertical' | 'both';

export type DragScrollOptions = {
  scrollZoneSize?: number;
  minScrollSpeed?: number;
  maxScrollSpeed?: number;
  axis?: DragScrollAxis;
  initialPointer?: { clientX: number; clientY: number };
  onScroll?: (clientX: number, clientY: number) => void;
  onScrollThrottleMs?: number;
  stickyCandidates?: HTMLElement[];
};

const isWindowTarget = (target: EventTarget): target is Window => {
  return typeof window !== 'undefined' && target === window;
};

const isElementTarget = (target: EventTarget): target is HTMLElement => {
  return target instanceof HTMLElement;
};

const isInsetSet = (value: string) => value !== '' && value !== 'auto';

const isTopAnchored = (style: CSSStyleDeclaration) => {
  return isInsetSet(style.top) || isInsetSet(style.insetBlockStart);
};

const getViewportContainer = (target: EventTarget): HTMLElement | null => {
  if (isElementTarget(target)) {
    return target;
  }

  if (isWindowTarget(target)) {
    return document.body;
  }

  return null;
};

const collectStickyCandidates = (target: EventTarget): HTMLElement[] => {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return [];
  }

  const container = getViewportContainer(target);
  if (!container) {
    return [];
  }

  const candidates: HTMLElement[] = [];
  const rootStyle = window.getComputedStyle(container);
  if ((rootStyle.position === 'sticky' || rootStyle.position === 'fixed') && isTopAnchored(rootStyle)) {
    candidates.push(container);
  }

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_ELEMENT);
  let current = walker.nextNode();
  while (current) {
    if (current instanceof HTMLElement) {
      const style = window.getComputedStyle(current);
      if ((style.position === 'sticky' || style.position === 'fixed') && isTopAnchored(style)) {
        candidates.push(current);
      }
    }
    current = walker.nextNode();
  }

  return candidates;
};

const getStickyTopBoundary = (
  rect: { top: number; bottom: number; left: number; right: number },
  stickyCandidates: HTMLElement[],
  topAnchorThresholdPx: number,
): number => {
  if (stickyCandidates.length === 0) {
    return rect.top;
  }

  let stickyTop = rect.top;

  for (const element of stickyCandidates) {
    const elementRect = element.getBoundingClientRect();
    const intersectsHorizontally = elementRect.right > rect.left && elementRect.left < rect.right;
    const intersectsTopZone = elementRect.bottom > rect.top && elementRect.top <= rect.top + topAnchorThresholdPx;

    if (!intersectsHorizontally || !intersectsTopZone) {
      continue;
    }

    stickyTop = Math.max(stickyTop, elementRect.bottom);
  }

  return Math.min(stickyTop, rect.bottom);
};

const getAdjustedRect = (
  rect: { top: number; bottom: number; left: number; right: number },
  stickyTop: number,
  scrollZoneSize: number,
): { top: number; bottom: number; left: number; right: number } => {
  const maxTopForBidirectionalScroll = rect.bottom - scrollZoneSize * 2;

  return {
    left: rect.left,
    right: rect.right,
    bottom: rect.bottom,
    top: Math.max(rect.top, Math.min(stickyTop, maxTopForBidirectionalScroll)),
  };
};

// NOTE: 드래그 중 끝에서 자동 스크롤
export function handleDragScroll(
  viewport: ScrollViewport | null,
  isDragging: boolean,
  options: DragScrollOptions = {},
): (() => void) | undefined {
  const {
    scrollZoneSize = 50,
    minScrollSpeed = 1,
    maxScrollSpeed = 15,
    axis = 'vertical',
    initialPointer,
    onScroll,
    onScrollThrottleMs = 50,
    stickyCandidates: providedStickyCandidates,
  } = options;

  if (!isDragging || !viewport) {
    return;
  }

  const useHorizontalScroll = axis === 'both';
  const stickyCandidates = providedStickyCandidates ?? collectStickyCandidates(viewport.target);
  const topAnchorThresholdPx = Math.max(scrollZoneSize * 2, 96);
  const toRect = (rect: { top: number; bottom: number; left: number; right: number }) => ({
    top: rect.top,
    bottom: rect.bottom,
    left: rect.left,
    right: rect.right,
  });
  const initialRawRect = toRect(viewport.getRect());
  const initialStickyTop = getStickyTopBoundary(initialRawRect, stickyCandidates, topAnchorThresholdPx);
  const stickyTopInset = Math.max(0, initialStickyTop - initialRawRect.top);
  const getStableStickyTop = (rawRect: { top: number; bottom: number; left: number; right: number }) => rawRect.top + stickyTopInset;

  let lastPointerX = 0;
  let lastPointerY = 0;
  let animationId: number | null = null;
  let lastOnScrollTime = 0;

  const isNearEdge = (rect: { top: number; bottom: number; left: number; right: number }) => {
    const isNearVertical = lastPointerY < rect.top + scrollZoneSize || lastPointerY > rect.bottom - scrollZoneSize;
    if (useHorizontalScroll) {
      const isNearHorizontal = lastPointerX < rect.left + scrollZoneSize || lastPointerX > rect.right - scrollZoneSize;
      return isNearVertical || isNearHorizontal;
    }
    return isNearVertical;
  };

  const getScrollSpeed = (distance: number) => {
    return Math.min(maxScrollSpeed, Math.max(minScrollSpeed, distance / 3));
  };

  const updatePointer = (clientX: number, clientY: number) => {
    lastPointerX = clientX;
    lastPointerY = clientY;

    const rawRect = toRect(viewport.getRect());
    const stickyTop = getStableStickyTop(rawRect);
    const rect = getAdjustedRect(rawRect, stickyTop, scrollZoneSize);

    if (!useHorizontalScroll && (lastPointerX < rect.left || lastPointerX > rect.right)) {
      return;
    }

    if (isNearEdge(rect) && animationId === null) {
      animationId = requestAnimationFrame(scroll);
    }
  };

  const handleDragOver = (e: DragEvent) => {
    updatePointer(e.clientX, e.clientY);
  };

  const handlePointerMove = (e: PointerEvent) => {
    updatePointer(e.clientX, e.clientY);
  };

  const scroll = () => {
    const rawRect = toRect(viewport.getRect());
    const stickyTop = getStableStickyTop(rawRect);
    const rect = getAdjustedRect(rawRect, stickyTop, scrollZoneSize);

    if (!useHorizontalScroll && (lastPointerX < rect.left || lastPointerX > rect.right)) {
      animationId = null;
      return;
    }

    const now = performance.now();
    const shouldCallOnScroll = now - lastOnScrollTime >= onScrollThrottleMs;

    let deltaX = 0;
    let deltaY = 0;

    if (lastPointerY < rect.top + scrollZoneSize) {
      const distance = rect.top + scrollZoneSize - lastPointerY;
      deltaY = -getScrollSpeed(distance);
    } else if (lastPointerY > rect.bottom - scrollZoneSize) {
      const distance = lastPointerY - (rect.bottom - scrollZoneSize);
      deltaY = getScrollSpeed(distance);
    }

    if (useHorizontalScroll) {
      if (lastPointerX < rect.left + scrollZoneSize) {
        const distance = rect.left + scrollZoneSize - lastPointerX;
        deltaX = -getScrollSpeed(distance);
      } else if (lastPointerX > rect.right - scrollZoneSize) {
        const distance = lastPointerX - (rect.right - scrollZoneSize);
        deltaX = getScrollSpeed(distance);
      }
    }

    if (deltaX === 0 && deltaY === 0) {
      animationId = null;
      return;
    }

    const prevScrollTop = viewport.getScrollTop();
    const prevScrollLeft = viewport.getScrollLeft();
    viewport.scrollBy(deltaX, deltaY);

    if (shouldCallOnScroll && (viewport.getScrollTop() !== prevScrollTop || viewport.getScrollLeft() !== prevScrollLeft)) {
      lastOnScrollTime = now;
      onScroll?.(lastPointerX, lastPointerY);
    }

    animationId = requestAnimationFrame(scroll);
  };

  viewport.target.addEventListener('dragover', handleDragOver as EventListener);
  viewport.target.addEventListener('pointermove', handlePointerMove as EventListener);
  if (initialPointer) {
    updatePointer(initialPointer.clientX, initialPointer.clientY);
  }

  return () => {
    viewport.target.removeEventListener('dragover', handleDragOver as EventListener);
    viewport.target.removeEventListener('pointermove', handlePointerMove as EventListener);
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }
  };
}
