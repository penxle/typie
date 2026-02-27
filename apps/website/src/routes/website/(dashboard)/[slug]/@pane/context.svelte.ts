import { LocalStore } from '@typie/ui/state';
import { getContext, setContext } from 'svelte';
import { hitTest, resolveDrop } from './dnd';
import { addPane, collectPanes, findAdjacentPane, findMemberById, movePane, removePane, replacePane, swapPanes } from './tree';
import type { AppContext } from '@typie/ui/context';
import type { DragItem, DragPane, DropZone, Pane, PaneGroup, PaneGroupState, Rect } from './types';

export type {
  DragItem,
  DragPane,
  DropZone,
  Member,
  Pane,
  PaneAxis,
  PaneGroup,
  PaneGroupState,
  PaneInit,
  PanelTab,
  PanePlacement,
  PaneSide,
  Rect,
} from './types';

const key: unique symbol = Symbol('PaneGroup');

export const getPaneGroup = () => {
  return getContext<PaneGroup>(key);
};

export const setupPaneGroup = (app: AppContext) => {
  const userId = app.userId;

  let activeZone = $state<{ paneId: string; dropZone: DropZone } | null>(null);
  let draggingPaneId = $state<string | null>(null);
  const findReplaceOpenByPaneId = $state<Record<string, boolean>>({});
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperatively read, not reactive state
  const paneRects = new Map<string, Rect>();

  const state = new LocalStore<PaneGroupState>(`typie:panegroup:${userId}`, {
    root: null,
    focusedPaneId: null,
    panelExpandedByPaneId: {},
    panelTabByPaneId: {},
  });
  const panes = $derived(collectPanes(state.current.root));

  const context: PaneGroup = {
    state,
    get panes() {
      return panes;
    },
    get enabled() {
      return panes.length > 1;
    },
    addPane: (pane, placement) => {
      if (!context.state.current.root) return false;

      const result = addPane(context.state.current.root, pane, placement);
      if (!result) return false;

      context.state.current.root = result.root;
      context.state.current.focusedPaneId = result.focusedPaneId;

      return true;
    },
    movePane: (paneId, placement) => {
      if (!context.state.current.root) return false;

      const result = movePane(context.state.current.root, paneId, placement);
      if (!result) return false;

      context.state.current.root = result.root;
      context.state.current.focusedPaneId = result.focusedPaneId;

      return true;
    },
    swapPane: (firstPaneId, secondPaneId) => {
      if (!context.state.current.root) return false;

      const result = swapPanes(context.state.current.root, firstPaneId, secondPaneId);
      if (result === context.state.current.root) return false;

      context.state.current.root = result;
      context.state.current.focusedPaneId = firstPaneId;

      return true;
    },
    removePane: (paneId) => {
      if (!context.state.current.root) return false;

      const target = findMemberById(context.state.current.root, paneId);
      if (!target) return false;

      // 마지막 pane → home으로 교체 (마지막 home이면 거부)
      if (panes.length <= 1) {
        if (target.type === 'pane' && target.kind === 'home') return false;
        return context.replacePane(paneId, { kind: 'home' });
      }

      const adjacent = context.state.current.focusedPaneId === paneId ? findAdjacentPane(context.state.current.root, paneId) : null;

      context.state.current.root = removePane(context.state.current.root, paneId);

      if (context.state.current.focusedPaneId === paneId) {
        context.state.current.focusedPaneId = adjacent?.id ?? null;
      }

      // per-paneId 상태 cleanup
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete context.state.current.panelExpandedByPaneId[paneId];
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete context.state.current.panelTabByPaneId[paneId];
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete findReplaceOpenByPaneId[paneId];

      return true;
    },
    replacePane: (paneId, pane) => {
      if (!context.state.current.root) return false;

      const { root, newPaneId } = replacePane(context.state.current.root, paneId, pane);
      context.state.current.root = root;
      context.state.current.focusedPaneId = newPaneId;

      return true;
    },

    get findReplaceOpenByPaneId() {
      return findReplaceOpenByPaneId;
    },

    // drag-drop
    get activeZone() {
      return activeZone;
    },
    set activeZone(value) {
      activeZone = value;
    },
    get draggingPaneId() {
      return draggingPaneId;
    },
    set draggingPaneId(value) {
      draggingPaneId = value;
    },
    rootElement: null,
    paneRects,
    hitTest(x: number, y: number) {
      if (!context.rootElement) return null;
      return hitTest(context.paneRects, context.rootElement, x, y);
    },
    updateActiveZone(x: number, y: number) {
      activeZone = context.hitTest(x, y);
    },
    executeDrop(item: DragItem | DragPane) {
      if (!activeZone) return false;
      const result = resolveDrop(item, activeZone, context.state.current.root, context);
      activeZone = null;
      draggingPaneId = null;
      return result;
    },
    cancelDrag() {
      activeZone = null;
      draggingPaneId = null;
    },
  };

  setContext(key, context);

  return context;
};

export const setupPane = (pane: Pane) => {
  setContext('pane', pane);
};

export const getPane = () => {
  return getContext<Pane>('pane');
};
