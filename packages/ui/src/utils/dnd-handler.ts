import { token } from '@typie/styled-system/tokens';
import { thresholdDrag } from '../actions/threshold-drag.svelte';

export type Ghost = {
  element: HTMLElement;
  offsetX: number;
  offsetY: number;
};

const createGhost = (sourceElement: HTMLElement, x: number, y: number): Ghost => {
  const rect = sourceElement.getBoundingClientRect();
  const ghostElement = document.createElement('div');

  const cloned = sourceElement.cloneNode(true) as HTMLElement;
  (cloned as unknown as { inert: boolean }).inert = true;
  cloned.setAttribute('aria-hidden', 'true');
  cloned.style.pointerEvents = 'none';
  ghostElement.append(cloned);

  ghostElement.style.position = 'fixed';
  ghostElement.style.width = `${rect.width}px`;
  ghostElement.style.height = `${rect.height}px`;
  ghostElement.style.opacity = '0.8';
  ghostElement.style.pointerEvents = 'none';
  ghostElement.style.zIndex = token('zIndex.ghost');
  ghostElement.style.left = `${rect.left}px`;
  ghostElement.style.top = `${rect.top}px`;

  document.body.append(ghostElement);

  return {
    element: ghostElement,
    offsetX: x - rect.left,
    offsetY: y - rect.top,
  };
};

const updateGhost = (ghost: Ghost, x: number, y: number) => {
  ghost.element.style.left = `${x - ghost.offsetX}px`;
  ghost.element.style.top = `${y - ghost.offsetY}px`;
};

const removeGhost = (ghost: Ghost) => {
  ghost.element.remove();
};

export type DndHandlerOptions = {
  threshold?: number;

  getDragTarget?: (e: PointerEvent) => HTMLElement | null;

  canStartDrag?: (e: PointerEvent, target: HTMLElement) => boolean;

  onDragStart?: (e: PointerEvent, target: HTMLElement) => void;

  onDragMove?: (e: PointerEvent) => void;

  onDragEnd?: (e: PointerEvent) => void;

  onDragCancel?: () => void;

  showGhost?: boolean;

  excludeSelectors?: string[];

  dragHandleSelector?: string;
};

export type DndHandlerState = {
  isDragging: boolean;
  ghost: Ghost | null;
};

export const createDndHandler = (node: HTMLElement, options: DndHandlerOptions) => {
  const {
    threshold = 10,
    getDragTarget,
    canStartDrag,
    onDragStart,
    onDragMove,
    onDragEnd,
    onDragCancel,
    showGhost = true,
    excludeSelectors = ['button', '[role="button"]', 'a[href]', 'input', 'textarea', 'select'],
    dragHandleSelector,
  } = options;

  let dragging = false;
  let isDragActive = false;
  let ghost: Ghost | null = null;
  let hoveredTarget: HTMLElement | null = null;

  const updateCursor = (e: PointerEvent | null) => {
    if (dragging) {
      document.body.style.cursor = 'grabbing';
      return;
    }

    document.body.style.cursor = '';

    if (getDragTarget) {
      const target = e ? getDragTarget(e) : null;

      if (hoveredTarget && hoveredTarget !== target && hoveredTarget.isConnected) {
        hoveredTarget.style.cursor = '';
      }
      if (target?.isConnected) {
        target.style.cursor = 'grab';
      }
      hoveredTarget = target;
    } else {
      node.style.cursor = 'grab';
    }
  };

  const cleanup = () => {
    if (ghost) {
      removeGhost(ghost);
      ghost = null;
    }
    if (hoveredTarget) {
      if (hoveredTarget.isConnected) {
        hoveredTarget.style.cursor = '';
      }
      hoveredTarget = null;
    }
    dragging = false;
    isDragActive = false;
    updateCursor(null);
  };

  const drag = thresholdDrag<{ target: HTMLElement }>(node, {
    threshold,
    start: (e) => {
      if (e.button !== 0 || !e.isPrimary) return null;

      const target = e.target as HTMLElement;

      if (dragHandleSelector && !target.closest(dragHandleSelector)) {
        return null;
      }

      if (excludeSelectors.some((selector) => target.closest(selector))) {
        return null;
      }

      const extractedTarget = getDragTarget ? getDragTarget(e) : node;
      if (!extractedTarget) return null;

      if (canStartDrag && !canStartDrag(e, extractedTarget)) {
        return null;
      }

      dragging = true;
      updateCursor(e);
      return { target: extractedTarget };
    },
    activate: ({ target }, startEvent) => {
      isDragActive = true;
      if (showGhost) {
        ghost = createGhost(target, startEvent.clientX, startEvent.clientY);
      }
      onDragStart?.(startEvent, target);
      return true;
    },
    move: (_session, e) => {
      if (ghost) {
        updateGhost(ghost, e.clientX, e.clientY);
      }
      onDragMove?.(e);
    },
    press: cleanup,
    end: (_session, e) => {
      cleanup();
      onDragEnd?.(e);
    },
    cancel: (_session, _reason, wasActive) => {
      cleanup();
      if (wasActive) onDragCancel?.();
    },
  });

  const handlePointerHoverMove = (e: PointerEvent) => {
    if (dragging) return;
    updateCursor(e);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!(e.key === 'Escape' && dragging)) {
      return;
    }

    drag.cancel();
  };

  node.addEventListener('pointermove', handlePointerHoverMove);
  window.addEventListener('keydown', handleKeyDown);

  updateCursor(null);

  return {
    state: (): DndHandlerState => ({
      isDragging: dragging && isDragActive,
      ghost,
    }),
    destroy: () => {
      drag.destroy();
      node.removeEventListener('pointermove', handlePointerHoverMove);
      window.removeEventListener('keydown', handleKeyDown);
    },
  };
};
