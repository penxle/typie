import type { ScrollViewport } from './scroll-viewport';

export type DragScrollAxis = 'vertical' | 'both';

export type DragScrollDelta = {
  deltaX: number;
  deltaY: number;
};

export type DragScrollOptions = {
  axis?: DragScrollAxis;
  initialPointer?: { clientX: number; clientY: number };
  onScroll?: (clientX: number, clientY: number, delta: DragScrollDelta) => void;
  onScrollThrottleMs?: number;
  stickyCandidates?: HTMLElement[];
};

export type DragScroll = {
  updatePointer(clientX: number, clientY: number): void;
  destroy(): void;
};

const SCROLL_ZONE_SIZE_PX = 60;
const MINIMUM_SCROLL_SPEED_PX_PER_SECOND = 240;
const EDGE_SCROLL_SPEED_PX_PER_SECOND = 960;
const OUTSIDE_SCROLL_SPEED_GAIN_PER_SECOND = 30;
const MAXIMUM_SCROLL_SPEED_PX_PER_SECOND = 1800;
const MAX_FRAME_DELTA_MS = 100;

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

// 포인터를 받지 않는 오버레이는 상단 sticky 헤더가 아니다 — 그렇게 세면 뷰포트 전체를 덮는 장식 하나가
// 스크롤 존의 위 경계를 바닥까지 끌어내려, 아래 가장자리에서 끄는데 위로 튀는 역전이 난다
const isInteractive = (style: CSSStyleDeclaration) => style.pointerEvents !== 'none';

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
  if ((rootStyle.position === 'sticky' || rootStyle.position === 'fixed') && isTopAnchored(rootStyle) && isInteractive(rootStyle)) {
    candidates.push(container);
  }

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_ELEMENT);
  let current = walker.nextNode();
  while (current) {
    if (current instanceof HTMLElement) {
      const style = window.getComputedStyle(current);
      if ((style.position === 'sticky' || style.position === 'fixed') && isTopAnchored(style) && isInteractive(style)) {
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
): { top: number; bottom: number; left: number; right: number } => {
  const maxTopForBidirectionalScroll = rect.bottom - SCROLL_ZONE_SIZE_PX * 2;

  return {
    left: rect.left,
    right: rect.right,
    bottom: rect.bottom,
    top: Math.max(rect.top, Math.min(stickyTop, maxTopForBidirectionalScroll)),
  };
};

export function createDragScroll(viewport: ScrollViewport, options: DragScrollOptions = {}): DragScroll {
  const { axis = 'vertical', initialPointer, onScroll, onScrollThrottleMs = 50, stickyCandidates: providedStickyCandidates } = options;

  const useHorizontalScroll = axis === 'both';
  const stickyCandidates = providedStickyCandidates ?? collectStickyCandidates(viewport.target);
  const topAnchorThresholdPx = Math.max(SCROLL_ZONE_SIZE_PX * 2, 96);
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
  let lastFrameTime: number | null = null;
  let lastOnScrollTime = 0;
  let destroyed = false;

  const isNearEdge = (rect: { top: number; bottom: number; left: number; right: number }) => {
    const isNearVertical = lastPointerY < rect.top + SCROLL_ZONE_SIZE_PX || lastPointerY > rect.bottom - SCROLL_ZONE_SIZE_PX;
    if (useHorizontalScroll) {
      const isNearHorizontal = lastPointerX < rect.left + SCROLL_ZONE_SIZE_PX || lastPointerX > rect.right - SCROLL_ZONE_SIZE_PX;
      return isNearVertical || isNearHorizontal;
    }
    return isNearVertical;
  };

  const getScrollSpeed = (distanceToEdge: number) => {
    const insideProgress = Math.max(0, Math.min(1, (SCROLL_ZONE_SIZE_PX - distanceToEdge) / SCROLL_ZONE_SIZE_PX));
    const outsideDistance = Math.max(-distanceToEdge, 0);
    const insideSpeed =
      MINIMUM_SCROLL_SPEED_PX_PER_SECOND + insideProgress * (EDGE_SCROLL_SPEED_PX_PER_SECOND - MINIMUM_SCROLL_SPEED_PX_PER_SECOND);

    return Math.min(MAXIMUM_SCROLL_SPEED_PX_PER_SECOND, insideSpeed + outsideDistance * OUTSIDE_SCROLL_SPEED_GAIN_PER_SECOND);
  };

  const getScrollVelocity = (rect: { top: number; bottom: number; left: number; right: number }) => {
    let velocityX = 0;
    let velocityY = 0;

    if (lastPointerY < rect.top + SCROLL_ZONE_SIZE_PX) {
      velocityY = -getScrollSpeed(lastPointerY - rect.top);
    } else if (lastPointerY > rect.bottom - SCROLL_ZONE_SIZE_PX) {
      velocityY = getScrollSpeed(rect.bottom - lastPointerY);
    }

    if (useHorizontalScroll) {
      if (lastPointerX < rect.left + SCROLL_ZONE_SIZE_PX) {
        velocityX = -getScrollSpeed(lastPointerX - rect.left);
      } else if (lastPointerX > rect.right - SCROLL_ZONE_SIZE_PX) {
        velocityX = getScrollSpeed(rect.right - lastPointerX);
      }
    }

    return { velocityX, velocityY };
  };

  const updatePointer = (clientX: number, clientY: number) => {
    if (destroyed) {
      return;
    }

    lastPointerX = clientX;
    lastPointerY = clientY;

    const rawRect = toRect(viewport.getRect());
    const stickyTop = getStableStickyTop(rawRect);
    const rect = getAdjustedRect(rawRect, stickyTop);

    if (!useHorizontalScroll && (lastPointerX < rect.left || lastPointerX > rect.right)) {
      return;
    }

    if (animationId === null && isNearEdge(rect)) {
      animationId = requestAnimationFrame(scroll);
    }
  };

  const scroll = (frameTime: number) => {
    if (destroyed) {
      animationId = null;
      lastFrameTime = null;
      return;
    }

    const rawRect = toRect(viewport.getRect());
    const stickyTop = getStableStickyTop(rawRect);
    const rect = getAdjustedRect(rawRect, stickyTop);

    if (!useHorizontalScroll && (lastPointerX < rect.left || lastPointerX > rect.right)) {
      animationId = null;
      lastFrameTime = null;
      return;
    }

    const { velocityX, velocityY } = getScrollVelocity(rect);

    if (velocityX === 0 && velocityY === 0) {
      animationId = null;
      lastFrameTime = null;
      return;
    }

    if (lastFrameTime === null) {
      lastFrameTime = frameTime;
      animationId = requestAnimationFrame(scroll);
      return;
    }

    const frameDeltaMs = frameTime - lastFrameTime;
    lastFrameTime = frameTime;
    if (frameDeltaMs > MAX_FRAME_DELTA_MS) {
      animationId = requestAnimationFrame(scroll);
      return;
    }

    const elapsedSeconds = frameDeltaMs / 1000;
    const shouldCallOnScroll = frameTime - lastOnScrollTime >= onScrollThrottleMs;

    const prevScrollTop = viewport.getScrollTop();
    const prevScrollLeft = viewport.getScrollLeft();
    viewport.scrollBy(velocityX * elapsedSeconds, velocityY * elapsedSeconds);

    const deltaY = viewport.getScrollTop() - prevScrollTop;
    const deltaX = viewport.getScrollLeft() - prevScrollLeft;
    const didScroll = deltaY !== 0 || deltaX !== 0;
    if (shouldCallOnScroll && didScroll) {
      lastOnScrollTime = frameTime;
      onScroll?.(lastPointerX, lastPointerY, { deltaX, deltaY });
    }

    if (!destroyed) {
      animationId = requestAnimationFrame(scroll);
    }
  };

  if (initialPointer) {
    updatePointer(initialPointer.clientX, initialPointer.clientY);
  }

  return {
    updatePointer,
    destroy: () => {
      if (destroyed) {
        return;
      }

      destroyed = true;
      if (animationId !== null) {
        cancelAnimationFrame(animationId);
        animationId = null;
      }
      lastFrameTime = null;
    },
  };
}

// NOTE: 드래그 중 끝에서 자동 스크롤
export function handleDragScroll(
  viewport: ScrollViewport | null,
  isDragging: boolean,
  options: DragScrollOptions = {},
): (() => void) | undefined {
  if (!isDragging || !viewport) {
    return;
  }

  const dragScroll = createDragScroll(viewport, options);
  const handleDragOver = (e: DragEvent) => {
    dragScroll.updatePointer(e.clientX, e.clientY);
  };
  const handlePointerMove = (e: PointerEvent) => {
    dragScroll.updatePointer(e.clientX, e.clientY);
  };

  viewport.target.addEventListener('dragover', handleDragOver as EventListener);
  viewport.target.addEventListener('pointermove', handlePointerMove as EventListener);

  return () => {
    viewport.target.removeEventListener('dragover', handleDragOver as EventListener);
    viewport.target.removeEventListener('pointermove', handlePointerMove as EventListener);
    dragScroll.destroy();
  };
}
