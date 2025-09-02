import { token } from '@typie/styled-system/tokens';
import type { Action } from 'svelte/action';
import type { DragDropContext, DragView } from './drag-context.svelte';

type DragViewOptions = {
  dragDropContext: DragDropContext;
  viewId: string;
};

type Ghost = {
  element: HTMLElement;
  offsetX: number;
  offsetY: number;
};

export const dragView: Action<HTMLElement, DragViewOptions> = (node, options) => {
  let dragging = false;
  let dragStartPos = { x: 0, y: 0 };
  let ghost: Ghost | null = null;
  let capturedPointerId: number | null = null;
  let animationFrameId: number | null = null;

  const createGhost = (x: number, y: number) => {
    const rect = node.getBoundingClientRect();
    const ghostElement = document.createElement('div');

    const cloned = node.cloneNode(true) as HTMLElement;
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

    ghost = {
      element: ghostElement,
      offsetX: x - rect.left,
      offsetY: y - rect.top,
    };
  };

  const updateGhost = (x: number, y: number) => {
    if (ghost) {
      ghost.element.style.left = `${x - ghost.offsetX}px`;
      ghost.element.style.top = `${y - ghost.offsetY}px`;
    }
  };

  const removeGhost = () => {
    if (ghost) {
      ghost.element.remove();
      ghost = null;
    }
  };

  const updateCursor = () => {
    node.style.cursor = dragging ? 'grabbing' : 'grab';
  };

  const handlePointerCancel = (e: PointerEvent) => {
    if (dragging) {
      dragging = false;
      node.releasePointerCapture(e.pointerId);
      capturedPointerId = null;
      removeGhost();
      updateCursor();

      if (options.dragDropContext.state.isDragging) {
        options.dragDropContext.cancelDrag();
      }
    }
  };

  const handlePointerDown = (e: PointerEvent) => {
    // 주 버튼/프라이머리 포인터만 허용
    if (e.button !== 0 || !e.isPrimary) return;

    const target = e.target as HTMLElement;
    const excludes = ['button', '[role="button"]', '[role="menu"]', 'a[href]', 'input', 'textarea', 'select'];
    if (excludes.some((selector) => target.closest(selector))) {
      return;
    }

    dragging = true;
    dragStartPos = { x: e.clientX, y: e.clientY };
    node.setPointerCapture(e.pointerId);
    capturedPointerId = e.pointerId;
    updateCursor();
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!dragging) return;

    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    animationFrameId = requestAnimationFrame(() => {
      const distance = Math.sqrt(Math.pow(e.clientX - dragStartPos.x, 2) + Math.pow(e.clientY - dragStartPos.y, 2));

      if (distance > 10 && !options.dragDropContext.state.isDragging) {
        const dragItem: DragView = {
          type: 'view',
          viewId: options.viewId,
        };
        options.dragDropContext.startDrag(dragItem);
        createGhost(dragStartPos.x, dragStartPos.y);
      }

      if (options.dragDropContext.state.isDragging) {
        updateGhost(e.clientX, e.clientY);
      }
      animationFrameId = null;
    });
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (dragging) {
      dragging = false;
      node.releasePointerCapture(e.pointerId);
      capturedPointerId = null;
      removeGhost();
      updateCursor();

      if (options.dragDropContext.state.isDragging) {
        options.dragDropContext.drop();
      }
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && dragging) {
      dragging = false;
      if (capturedPointerId !== null) {
        node.releasePointerCapture(capturedPointerId);
        capturedPointerId = null;
      }
      removeGhost();
      updateCursor();
      if (options.dragDropContext.state.isDragging) {
        options.dragDropContext.cancelDrag();
      }
    }
  };

  node.addEventListener('pointercancel', handlePointerCancel);
  node.addEventListener('pointerdown', handlePointerDown);
  node.addEventListener('pointermove', handlePointerMove);
  node.addEventListener('pointerup', handlePointerUp);
  window.addEventListener('keydown', handleKeyDown);

  updateCursor();

  return {
    update(newOptions: DragViewOptions) {
      options = newOptions;
    },
    destroy() {
      if (animationFrameId) cancelAnimationFrame(animationFrameId);
      node.removeEventListener('pointercancel', handlePointerCancel);
      node.removeEventListener('pointerdown', handlePointerDown);
      node.removeEventListener('pointermove', handlePointerMove);
      node.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('keydown', handleKeyDown);
      removeGhost();
    },
  };
};
