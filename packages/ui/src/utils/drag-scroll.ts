export type DragScrollOptions = {
  scrollZoneSize?: number;
  maxScrollSpeed?: number;
  onScroll?: () => void;
};

// NOTE: 드래그 중 위, 아래 끝에서 자동 스크롤
export function handleDragScroll(
  scrollContainer: HTMLElement | null,
  isDragging: boolean,
  options: DragScrollOptions = {},
): (() => void) | undefined {
  if (!isDragging || !scrollContainer) return;

  const { scrollZoneSize = 50, maxScrollSpeed = 15, onScroll } = options;

  let lastPointerX = 0;
  let lastPointerY = 0;
  let animationId: number | null = null;

  const updatePointer = (clientX: number, clientY: number) => {
    lastPointerX = clientX;
    lastPointerY = clientY;

    const containerRect = scrollContainer.getBoundingClientRect();

    if (lastPointerX < containerRect.left || lastPointerX > containerRect.right) {
      return;
    }

    if (
      (lastPointerY < containerRect.top + scrollZoneSize || lastPointerY > containerRect.bottom - scrollZoneSize) &&
      animationId === null
    ) {
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
    const containerRect = scrollContainer.getBoundingClientRect();

    if (lastPointerX < containerRect.left || lastPointerX > containerRect.right) {
      animationId = null;
      return;
    }

    if (lastPointerY < containerRect.top + scrollZoneSize) {
      const distance = containerRect.top + scrollZoneSize - lastPointerY;
      const scrollSpeed = Math.min(maxScrollSpeed, Math.max(1, distance / 3));
      scrollContainer.scrollBy(0, -scrollSpeed);
      onScroll?.();
      animationId = requestAnimationFrame(scroll);
    } else if (lastPointerY > containerRect.bottom - scrollZoneSize) {
      const distance = lastPointerY - (containerRect.bottom - scrollZoneSize);
      const scrollSpeed = Math.min(maxScrollSpeed, Math.max(1, distance / 3));
      scrollContainer.scrollBy(0, scrollSpeed);
      onScroll?.();
      animationId = requestAnimationFrame(scroll);
    } else {
      animationId = null;
    }
  };

  scrollContainer.addEventListener('dragover', handleDragOver);
  scrollContainer.addEventListener('pointermove', handlePointerMove);

  return () => {
    scrollContainer.removeEventListener('dragover', handleDragOver);
    scrollContainer.removeEventListener('pointermove', handlePointerMove);
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
    }
  };
}
