import mixpanel from 'mixpanel-browser';
import { findMemberById } from './tree';
import type { Action } from 'svelte/action';
import type { DragItem, DragPane, DropZone, Member, PaneGroup, PaneInit, PanePlacement, Rect } from './types';

export const computeDropZone = (x: number, y: number, rect: Rect): DropZone => {
  const relX = x - rect.left;
  const relY = y - rect.top;
  const centerMargin = 0.3;

  const distLeft = relX;
  const distRight = rect.width - relX;
  const distTop = relY;
  const distBottom = rect.height - relY;

  if (
    distLeft > rect.width * centerMargin &&
    distRight > rect.width * centerMargin &&
    distTop > rect.height * centerMargin &&
    distBottom > rect.height * centerMargin
  ) {
    return 'center';
  }

  const min = Math.min(distLeft, distRight, distTop, distBottom);
  if (min === distLeft) return 'left';
  if (min === distRight) return 'right';
  if (min === distTop) return 'top';
  return 'bottom';
};

export const hitTest = (
  paneRects: Map<string, Rect>,
  rootElement: HTMLElement,
  x: number,
  y: number,
): { paneId: string; dropZone: DropZone } | null => {
  const rootRect = rootElement.getBoundingClientRect();
  const localX = x - rootRect.left;
  const localY = y - rootRect.top;

  let closestPaneId: string | null = null;
  let closestDist = Infinity;

  for (const [paneId, rect] of paneRects) {
    const dx = Math.max(rect.left - localX, 0, localX - (rect.left + rect.width));
    const dy = Math.max(rect.top - localY, 0, localY - (rect.top + rect.height));

    if (dx === 0 && dy === 0) {
      return { paneId, dropZone: computeDropZone(localX, localY, rect) };
    }

    const dist = dx + dy;
    if (dist < closestDist) {
      closestDist = dist;
      closestPaneId = paneId;
    }
  }

  if (closestPaneId && closestDist <= 8) {
    const rect = paneRects.get(closestPaneId);
    if (rect) {
      return { paneId: closestPaneId, dropZone: computeDropZone(localX, localY, rect) };
    }
  }

  return null;
};

type DropOps = {
  swapPane: (firstPaneId: string, secondPaneId: string) => boolean;
  addPane: (pane: PaneInit, placement: PanePlacement) => boolean;
  movePane: (paneId: string, placement: PanePlacement) => boolean;
  replacePane: (paneId: string, pane: PaneInit) => boolean;
};

export const resolveDrop = (
  item: DragItem | DragPane,
  zone: { paneId: string; dropZone: DropZone },
  root: Member | null,
  ops: DropOps,
): boolean => {
  const { paneId: targetPaneId, dropZone } = zone;

  if (item.type === 'pane') {
    if (dropZone === 'center') {
      if (item.paneId === targetPaneId) return false;
      ops.swapPane(item.paneId, targetPaneId);
      mixpanel.track('move_pane', { via: 'drag-drop', action: 'replace' });
    } else {
      if (item.paneId === targetPaneId) {
        const found = root ? findMemberById(root, item.paneId) : null;
        if (found?.type === 'pane') {
          ops.addPane(found, { paneId: targetPaneId, side: dropZone });
          mixpanel.track('duplicate_pane', { via: 'drag-drop', side: dropZone });
        }
      } else {
        ops.movePane(item.paneId, { paneId: targetPaneId, side: dropZone });
        mixpanel.track('move_pane', { via: 'drag-drop', action: 'add', side: dropZone });
      }
    }
  } else {
    if (dropZone === 'center') {
      ops.replacePane(targetPaneId, { kind: 'entity', slug: item.slug });
      mixpanel.track('replace_pane', { via: 'drag-drop' });
    } else {
      ops.addPane({ kind: 'entity', slug: item.slug }, { paneId: targetPaneId, side: dropZone });
      mixpanel.track('add_pane', { via: 'drag-drop', side: dropZone });
    }
  }

  return true;
};

type DragPaneOptions = {
  paneGroup: PaneGroup;
  paneId: string;
};

export const dragPane: Action<HTMLElement, DragPaneOptions> = (node, options) => {
  let isDragging = false;
  let holdActivated = false;
  let holdTimer: ReturnType<typeof setTimeout> | null = null;
  let activePointerId = -1;
  let startX = 0;
  let startY = 0;
  const HOLD_DURATION = 300;
  const IMMEDIATE_DRAG_THRESHOLD = 8;
  const POST_HOLD_DRAG_THRESHOLD = 5;

  node.style.cursor = 'grab';
  node.style.touchAction = 'none';

  const clearHold = () => {
    if (holdTimer) {
      clearTimeout(holdTimer);
      holdTimer = null;
    }
  };

  const resetState = () => {
    clearHold();
    isDragging = false;
    holdActivated = false;
    activePointerId = -1;
    options.paneGroup.draggingPaneId = null;
    node.style.cursor = 'grab';
    document.body.style.cursor = '';
  };

  const handlePointerDown = (e: PointerEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button, [role="button"], [role="menu"], a[href], input, textarea, select')) {
      return;
    }

    e.preventDefault();
    clearHold();

    startX = e.clientX;
    startY = e.clientY;
    activePointerId = e.pointerId;
    node.setPointerCapture(e.pointerId);

    holdActivated = false;
    holdTimer = setTimeout(() => {
      holdTimer = null;
      if (!node.hasPointerCapture(activePointerId)) return;
      holdActivated = true;
      options.paneGroup.draggingPaneId = options.paneId;
      node.style.cursor = 'grabbing';
      document.body.style.cursor = 'grabbing';
    }, HOLD_DURATION);
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!node.hasPointerCapture(e.pointerId)) return;

    const dist = Math.abs(e.clientX - startX) + Math.abs(e.clientY - startY);

    if (!holdActivated) {
      if (dist > IMMEDIATE_DRAG_THRESHOLD) {
        clearHold();
        holdActivated = true;
        isDragging = true;
        options.paneGroup.draggingPaneId = options.paneId;
        node.style.cursor = 'grabbing';
        document.body.style.cursor = 'grabbing';
      } else {
        return;
      }
    }

    if (!isDragging) {
      if (dist > POST_HOLD_DRAG_THRESHOLD) {
        isDragging = true;
        node.style.cursor = 'grabbing';
        document.body.style.cursor = 'grabbing';
      } else {
        return;
      }
    }

    options.paneGroup.updateActiveZone(e.clientX, e.clientY);
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (isDragging) {
      const dragItem: DragPane = { type: 'pane', paneId: options.paneId };
      options.paneGroup.executeDrop(dragItem);
    }

    resetState();
    if (node.hasPointerCapture(e.pointerId)) {
      node.releasePointerCapture(e.pointerId);
    }
  };

  const handlePointerCancel = (e: PointerEvent) => {
    resetState();
    options.paneGroup.cancelDrag();
    if (node.hasPointerCapture(e.pointerId)) {
      node.releasePointerCapture(e.pointerId);
    }
  };

  const handleLostPointerCapture = () => {
    resetState();
    options.paneGroup.cancelDrag();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && (holdActivated || holdTimer)) {
      options.paneGroup.cancelDrag();
      const capturedPointerId = activePointerId;
      resetState();
      if (capturedPointerId !== -1 && node.hasPointerCapture(capturedPointerId)) {
        node.releasePointerCapture(capturedPointerId);
      }
    }
  };

  node.addEventListener('pointerdown', handlePointerDown);
  node.addEventListener('pointermove', handlePointerMove);
  node.addEventListener('pointerup', handlePointerUp);
  node.addEventListener('pointercancel', handlePointerCancel);
  node.addEventListener('lostpointercapture', handleLostPointerCapture);
  document.addEventListener('keydown', handleKeyDown);

  return {
    update(newOptions: DragPaneOptions) {
      options = newOptions;
    },
    destroy() {
      clearHold();
      node.removeEventListener('pointerdown', handlePointerDown);
      node.removeEventListener('pointermove', handlePointerMove);
      node.removeEventListener('pointerup', handlePointerUp);
      node.removeEventListener('pointercancel', handlePointerCancel);
      node.removeEventListener('lostpointercapture', handleLostPointerCapture);
      document.removeEventListener('keydown', handleKeyDown);
    },
  };
};
