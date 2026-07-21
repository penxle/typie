import { pointerCapture } from '@typie/ui/actions';
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

type DragPaneSession = {
  startX: number;
  startY: number;
};

export const dragPane: Action<HTMLElement, DragPaneOptions> = (node, options) => {
  let isDragging = false;
  let holdActivated = false;
  let holdTimer: ReturnType<typeof setTimeout> | null = null;
  let activeSession: DragPaneSession | null = null;
  const HOLD_DURATION = 300;
  const IMMEDIATE_DRAG_THRESHOLD = 8;
  const POST_HOLD_DRAG_THRESHOLD = 5;

  node.style.cursor = 'grab';
  node.style.touchAction = 'none';

  const clearHold = () => {
    if (!holdTimer) {
      return;
    }

    clearTimeout(holdTimer);
    holdTimer = null;
  };

  const resetState = () => {
    clearHold();
    isDragging = false;
    holdActivated = false;
    activeSession = null;
    options.paneGroup.draggingPaneId = null;
    node.style.cursor = 'grab';
    document.body.style.cursor = '';
  };

  const handlePointerDown = (e: PointerEvent): DragPaneSession | null => {
    if (e.button !== 0 || !e.isPrimary) return null;

    const target = e.target as HTMLElement;
    if (target.closest('button, [role="button"], [role="menu"], a[href], input, textarea, select')) {
      return null;
    }

    e.preventDefault();
    clearHold();

    const session = { startX: e.clientX, startY: e.clientY };
    activeSession = session;

    holdActivated = false;
    holdTimer = setTimeout(() => {
      holdTimer = null;
      if (activeSession !== session) return;
      holdActivated = true;
      options.paneGroup.draggingPaneId = options.paneId;
      node.style.cursor = 'grabbing';
      document.body.style.cursor = 'grabbing';
    }, HOLD_DURATION);
    return session;
  };

  const handlePointerMove = (session: DragPaneSession, e: PointerEvent) => {
    if (activeSession !== session) return;

    const dist = Math.abs(e.clientX - session.startX) + Math.abs(e.clientY - session.startY);

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

  const handlePointerUp = () => {
    if (isDragging) {
      const dragItem: DragPane = { type: 'pane', paneId: options.paneId };
      options.paneGroup.executeDrop(dragItem);
    }

    resetState();
  };

  const handlePointerCancel = () => {
    resetState();
    options.paneGroup.cancelDrag();
  };

  const capture = pointerCapture(node, {
    start: handlePointerDown,
    move: handlePointerMove,
    end: handlePointerUp,
    cancel: handlePointerCancel,
  });

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && (holdActivated || holdTimer)) {
      capture.cancel();
    }
  };

  document.addEventListener('keydown', handleKeyDown);

  return {
    update(newOptions: DragPaneOptions) {
      options = newOptions;
    },
    destroy() {
      capture.destroy();
      document.removeEventListener('keydown', handleKeyDown);
    },
  };
};
