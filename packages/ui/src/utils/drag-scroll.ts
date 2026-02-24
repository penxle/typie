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
};

// NOTE: 드래그 중 끝에서 자동 스크롤
export function handleDragScroll(
  viewport: ScrollViewport | null,
  isDragging: boolean,
  options: DragScrollOptions = {},
): (() => void) | undefined {
  if (!isDragging || !viewport) return;

  const {
    scrollZoneSize = 50,
    minScrollSpeed = 1,
    maxScrollSpeed = 15,
    axis = 'vertical',
    initialPointer,
    onScroll,
    onScrollThrottleMs = 50,
  } = options;

  const useHorizontalScroll = axis === 'both';

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

    const rect = viewport.getRect();

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
    const rect = viewport.getRect();

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
