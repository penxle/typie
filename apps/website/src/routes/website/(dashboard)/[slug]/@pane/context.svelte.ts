import { LocalStore } from '@typie/ui/state';
import { nanoid } from 'nanoid';
import { getContext, setContext } from 'svelte';
import { browser } from '$app/environment';
import { hitTest, resolveDrop } from './dnd';
import { addPane, collectPanes, findAdjacentPane, findMemberById, movePane, removePane, replacePane, swapPanes } from './tree';
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

const defaultPaneGroupState: PaneGroupState = {
  root: null,
  focusedPaneId: null,
  panelExpandedByPaneId: {},
  panelTabByPaneId: {},
};

type PaneGroupOptions = {
  userId: string;
  navigate: (path: string, options?: { replaceState?: boolean; keepFocus?: boolean }) => void;
  onSiteChange: (siteId: string) => void;
};

const migrateLegacyPaneGroup = (userId: string, siteId: string) => {
  const oldKey = `typie:panegroup:${userId}`;
  const newKey = `typie:panegroup:${siteId}`;

  const oldData = localStorage.getItem(oldKey);
  if (oldData && !localStorage.getItem(newKey)) {
    localStorage.setItem(newKey, oldData);
  }

  localStorage.removeItem(oldKey);
};

export const setupPaneGroup = (initialSiteId: string, options: PaneGroupOptions) => {
  if (browser) {
    migrateLegacyPaneGroup(options.userId, initialSiteId);
  }
  let resizing = $state(false);
  let activeZone = $state<{ paneId: string; dropZone: DropZone } | null>(null);
  let draggingPaneId = $state<string | null>(null);
  const findReplaceOpenByPaneId = $state<Record<string, boolean>>({});
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperatively read, not reactive state
  const paneRects = new Map<string, Rect>();

  let currentKey = `typie:panegroup:${initialSiteId}`;
  let currentSiteId = initialSiteId;
  let lastNavigatedSlug: string | undefined;

  const state = new LocalStore<PaneGroupState>(currentKey, defaultPaneGroupState);

  for (const k of Object.keys(state.current.panelTabByPaneId)) {
    if ((state.current.panelTabByPaneId[k] as string) === 'remarks') {
      state.current.panelTabByPaneId[k] = 'comment';
    }
  }

  const panes = $derived(collectPanes(state.current.root));

  const syncUrl = () => {
    const focusedPaneId = state.current.focusedPaneId;
    if (!focusedPaneId) return;

    const focusedPane = panes.find((p) => p.id === focusedPaneId);
    const targetSlug = focusedPane?.kind === 'entity' ? focusedPane.slug : focusedPane?.kind === 'home' ? 'home' : null;

    if (!targetSlug || targetSlug === lastNavigatedSlug) return;

    lastNavigatedSlug = targetSlug;
    options.navigate(`/${targetSlug}`, { replaceState: true, keepFocus: true });
  };

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

      syncUrl();
      return true;
    },
    movePane: (paneId, placement) => {
      if (!context.state.current.root) return false;

      const result = movePane(context.state.current.root, paneId, placement);
      if (!result) return false;

      context.state.current.root = result.root;
      context.state.current.focusedPaneId = result.focusedPaneId;

      syncUrl();
      return true;
    },
    swapPane: (firstPaneId, secondPaneId) => {
      if (!context.state.current.root) return false;

      const result = swapPanes(context.state.current.root, firstPaneId, secondPaneId);
      if (result === context.state.current.root) return false;

      context.state.current.root = result;
      context.state.current.focusedPaneId = firstPaneId;

      syncUrl();
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

      syncUrl();
      return true;
    },
    replacePane: (paneId, pane) => {
      if (!context.state.current.root) return false;

      const { root, newPaneId } = replacePane(context.state.current.root, paneId, pane);
      context.state.current.root = root;
      context.state.current.focusedPaneId = newPaneId;

      if (paneId !== newPaneId) {
        const { panelExpandedByPaneId, panelTabByPaneId } = context.state.current;

        if (Object.hasOwn(panelExpandedByPaneId, paneId)) {
          panelExpandedByPaneId[newPaneId] = panelExpandedByPaneId[paneId];
          // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
          delete panelExpandedByPaneId[paneId];
        }

        if (Object.hasOwn(panelTabByPaneId, paneId)) {
          panelTabByPaneId[newPaneId] = panelTabByPaneId[paneId];
          // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
          delete panelTabByPaneId[paneId];
        }
      }

      syncUrl();
      return true;
    },

    get findReplaceOpenByPaneId() {
      return findReplaceOpenByPaneId;
    },

    handleNavigation(slug: string, siteId?: string) {
      if (slug === lastNavigatedSlug) return;
      lastNavigatedSlug = slug;

      // 크로스사이트 → 사이트 전환 (switchToSite가 pane tree 구성)
      if (siteId && siteId !== currentSiteId) {
        context.switchToSite(siteId, slug);
        return;
      }

      // 같은 사이트 → pane tree 업데이트
      const isHome = slug === 'home';

      if (state.current.root) {
        const focusedPaneId = state.current.focusedPaneId;
        const focusedPane = focusedPaneId ? panes.find((p) => p.id === focusedPaneId) : null;

        if (isHome) {
          if (focusedPane?.kind !== 'home' && focusedPaneId) {
            context.replacePane(focusedPaneId, { kind: 'home' });
          }
        } else {
          if (focusedPane?.kind === 'entity' && focusedPane.slug === slug) {
            // 이미 해당 slug 표시 중 → skip
          } else if (focusedPaneId) {
            context.replacePane(focusedPaneId, { kind: 'entity', slug });
          } else {
            const paneId = nanoid();
            state.current.root = {
              id: nanoid(),
              type: 'axis',
              direction: 'horizontal',
              children: [{ id: paneId, type: 'pane', kind: 'entity', slug }],
              flexes: [1],
            };
            state.current.focusedPaneId = paneId;
          }
        }
      } else {
        const paneId = nanoid();
        state.current.root = {
          id: nanoid(),
          type: 'axis',
          direction: 'horizontal',
          children: [
            isHome ? { id: paneId, type: 'pane', kind: 'home' as const } : { id: paneId, type: 'pane', kind: 'entity' as const, slug },
          ],
          flexes: [1],
        };
        state.current.focusedPaneId = paneId;
      }
    },

    switchToSite(siteId: string, slug?: string) {
      const newKey = `typie:panegroup:${siteId}`;
      if (newKey === currentKey) return;
      currentKey = newKey;
      currentSiteId = siteId;

      state.switchKey(newKey, defaultPaneGroupState);

      if (!state.current.root) {
        const paneId = nanoid();
        const isHome = !slug;
        state.current.root = {
          id: nanoid(),
          type: 'axis',
          direction: 'horizontal',
          children: [
            isHome ? { id: paneId, type: 'pane', kind: 'home' as const } : { id: paneId, type: 'pane', kind: 'entity' as const, slug },
          ],
          flexes: [1],
        };
        state.current.focusedPaneId = paneId;
      } else if (slug) {
        const currentPanes = collectPanes(state.current.root);
        const existingPane = currentPanes.find((p) => p.kind === 'entity' && p.slug === slug);
        if (existingPane) {
          state.current.focusedPaneId = existingPane.id;
        } else if (state.current.focusedPaneId) {
          const result = replacePane(state.current.root, state.current.focusedPaneId, { kind: 'entity', slug });
          state.current.root = result.root;
          state.current.focusedPaneId = result.newPaneId;
        }
      }

      options.onSiteChange(siteId);
      syncUrl();
    },

    focusPane(paneId: string) {
      if (state.current.focusedPaneId === paneId) return;
      state.current.focusedPaneId = paneId;
      syncUrl();
    },

    get resizing() {
      return resizing;
    },
    set resizing(value) {
      resizing = value;
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
