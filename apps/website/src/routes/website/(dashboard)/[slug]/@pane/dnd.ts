import mixpanel from 'mixpanel-browser';
import { findMemberById } from './tree';
import type { Action } from 'svelte/action';
import type { DragItem, DragPane, DropZone, Member, PaneGroup, PaneInit, PanePlacement, Rect } from './types';

export const computeDropZone = (x: number, y: number, rect: Rect): DropZone => {
  const relX = x - rect.left;
  const relY = y - rect.top;
  const centerMargin = 0.3;

  if (relX < rect.width * centerMargin) return 'left';
  if (relX > rect.width * (1 - centerMargin)) return 'right';
  if (relY < rect.height * centerMargin) return 'top';
  if (relY > rect.height * (1 - centerMargin)) return 'bottom';
  return 'center';
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

  for (const [paneId, rect] of paneRects) {
    if (localX >= rect.left && localX <= rect.left + rect.width && localY >= rect.top && localY <= rect.top + rect.height) {
      return { paneId, dropZone: computeDropZone(localX, localY, rect) };
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
  let startX = 0;
  let startY = 0;
  const DRAG_THRESHOLD = 5;

  node.style.cursor = 'grab';

  const handlePointerDown = (e: PointerEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('button, [role="button"], [role="menu"], a[href], input, textarea, select')) {
      return;
    }

    startX = e.clientX;
    startY = e.clientY;
    node.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!node.hasPointerCapture(e.pointerId)) return;

    if (!isDragging) {
      if (Math.abs(e.clientX - startX) + Math.abs(e.clientY - startY) > DRAG_THRESHOLD) {
        isDragging = true;
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

    isDragging = false;
    document.body.style.cursor = '';
    if (node.hasPointerCapture(e.pointerId)) {
      node.releasePointerCapture(e.pointerId);
    }
  };

  const handlePointerCancel = (e: PointerEvent) => {
    isDragging = false;
    document.body.style.cursor = '';
    options.paneGroup.cancelDrag();
    if (node.hasPointerCapture(e.pointerId)) {
      node.releasePointerCapture(e.pointerId);
    }
  };

  node.addEventListener('pointerdown', handlePointerDown);
  node.addEventListener('pointermove', handlePointerMove);
  node.addEventListener('pointerup', handlePointerUp);
  node.addEventListener('pointercancel', handlePointerCancel);

  return {
    update(newOptions: DragPaneOptions) {
      options = newOptions;
    },
    destroy() {
      node.removeEventListener('pointerdown', handlePointerDown);
      node.removeEventListener('pointermove', handlePointerMove);
      node.removeEventListener('pointerup', handlePointerUp);
      node.removeEventListener('pointercancel', handlePointerCancel);
    },
  };
};
