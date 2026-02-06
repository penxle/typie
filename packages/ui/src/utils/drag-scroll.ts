import type { ScrollViewport } from './scroll-viewport';

export type DragScrollOptions = {
  scrollZoneSize?: number;
  maxScrollSpeed?: number;
  onScroll?: (clientX: number, clientY: number) => void;
  onScrollThrottleMs?: number;
};

// NOTE: 드래그 중 위, 아래 끝에서 자동 스크롤
export function handleDragScroll(
  viewport: ScrollViewport | null,
  isDragging: boolean,
  options: DragScrollOptions = {},
): (() => void) | undefined {
  if (!isDragging || !viewport) return;

  const { scrollZoneSize = 50, maxScrollSpeed = 15, onScroll, onScrollThrottleMs = 50 } = options;

  let lastPointerX = 0;
  let lastPointerY = 0;
  let animationId: number | null = null;
  let lastOnScrollTime = 0;

  const updatePointer = (clientX: number, clientY: number) => {
    lastPointerX = clientX;
    lastPointerY = clientY;

    const rect = viewport.getRect();

    if (lastPointerX < rect.left || lastPointerX > rect.right) {
      return;
    }

    if ((lastPointerY < rect.top + scrollZoneSize || lastPointerY > rect.bottom - scrollZoneSize) && animationId === null) {
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

    if (lastPointerX < rect.left || lastPointerX > rect.right) {
      animationId = null;
      return;
    }

    const now = performance.now();
    const shouldCallOnScroll = now - lastOnScrollTime >= onScrollThrottleMs;

    if (lastPointerY < rect.top + scrollZoneSize) {
      const distance = rect.top + scrollZoneSize - lastPointerY;
      const scrollSpeed = Math.min(maxScrollSpeed, Math.max(1, distance / 3));
      const prevScrollTop = viewport.getScrollTop();
      viewport.scrollBy(0, -scrollSpeed);
      if (shouldCallOnScroll && viewport.getScrollTop() !== prevScrollTop) {
        lastOnScrollTime = now;
        onScroll?.(lastPointerX, lastPointerY);
      }
      animationId = requestAnimationFrame(scroll);
    } else if (lastPointerY > rect.bottom - scrollZoneSize) {
      const distance = lastPointerY - (rect.bottom - scrollZoneSize);
      const scrollSpeed = Math.min(maxScrollSpeed, Math.max(1, distance / 3));
      const prevScrollTop = viewport.getScrollTop();
      viewport.scrollBy(0, scrollSpeed);
      if (shouldCallOnScroll && viewport.getScrollTop() !== prevScrollTop) {
        lastOnScrollTime = now;
        onScroll?.(lastPointerX, lastPointerY);
      }
      animationId = requestAnimationFrame(scroll);
    } else {
      animationId = null;
    }
  };

  viewport.target.addEventListener('dragover', handleDragOver as EventListener);
  viewport.target.addEventListener('pointermove', handlePointerMove as EventListener);

  return () => {
    viewport.target.removeEventListener('dragover', handleDragOver as EventListener);
    viewport.target.removeEventListener('pointermove', handlePointerMove as EventListener);
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }
  };
}
