import { LocalStore } from '@typie/ui/state';
import { getContext, setContext } from 'svelte';
import {
  addViewAtRoot,
  calculateViewPercentagesForNewView,
  closeSplitView,
  findViewById,
  getParentView,
  moveView,
  normalizeContainerPercentages,
  replaceSplitView,
} from './utils';

const key: unique symbol = Symbol('SplitViewContext');

export type SplitView =
  | {
      id: string;
      type: 'container';
      direction: 'horizontal' | 'vertical';
      children: SplitView[];
    }
  | SplitViewItem;

export type SplitViewItem = {
  id: string;
  type: 'item';
  slug: string;
};

export type SplitViewState = {
  view: SplitView | null;
  focusedViewId: string | null;
  enabled: boolean;
  basePercentages: Record<string, number>;
  currentPercentages: Record<string, number>;
};

type OmitFirst<T extends readonly unknown[]> = T extends readonly [unknown, ...infer R] ? R : [];

type SplitViewContext = {
  state: LocalStore<SplitViewState>;
  addViewAtRoot: (...args: OmitFirst<Parameters<typeof addViewAtRoot>>) => boolean;
  moveView: (...args: OmitFirst<Parameters<typeof moveView>>) => boolean;
  swapView: (firstViewId: string, secondViewId: string) => boolean;
  closeSplitView: (viewId: string) => boolean;
  replaceSplitView: (...args: OmitFirst<Parameters<typeof replaceSplitView>>) => boolean;
};

export const getSplitViewContext = () => {
  return getContext<SplitViewContext>(key);
};

export const setupSplitViewContext = (userId: string) => {
  const normalizeViewPercentages = (view: SplitView) => {
    if (view.type === 'container') {
      const normalized = normalizeContainerPercentages(view, context.state.current.currentPercentages);
      context.state.current.currentPercentages = {
        ...context.state.current.currentPercentages,
        ...normalized,
      };
      context.state.current.basePercentages = {
        ...context.state.current.basePercentages,
        ...normalized,
      };
    }
  };
  const updateViewPercentages = (splitViews: SplitView, focusedSplitViewId: string, sourceParentId?: string | null) => {
    const parentView = getParentView(splitViews, focusedSplitViewId);
    if (parentView && parentView.type === 'container') {
      const newPercentages = calculateViewPercentagesForNewView(parentView, focusedSplitViewId, context.state.current.currentPercentages);

      context.state.current.currentPercentages = {
        ...context.state.current.currentPercentages,
        ...newPercentages,
      };

      context.state.current.basePercentages = {
        ...context.state.current.basePercentages,
        ...newPercentages,
      };
    }

    if (sourceParentId) {
      const sourceParent = findViewById(splitViews, sourceParentId);
      if (sourceParent) {
        normalizeViewPercentages(sourceParent);
      }
    }
  };

  const context: SplitViewContext = {
    state: new LocalStore<SplitViewState>(`typie:splitview:${userId}`, {
      view: null,
      enabled: false,
      focusedViewId: null,
      basePercentages: {},
      currentPercentages: {},
    }),
    addViewAtRoot: (slug, direction) => {
      if (!context.state.current.view) return false;

      const result = addViewAtRoot(context.state.current.view, slug, direction);

      context.state.current.view = result.splitViews;
      context.state.current.focusedViewId = result.focusedSplitViewId;

      updateViewPercentages(result.splitViews, result.focusedSplitViewId);

      return true;
    },
    moveView: (source, target) => {
      if (!context.state.current.view) return false;

      const sourceParentId =
        'viewId' in source && source.delete ? (getParentView(context.state.current.view, source.viewId)?.id ?? null) : null;

      const result = moveView(context.state.current.view, source, target);
      if (!result) return false;

      context.state.current.view = result.splitViews;
      context.state.current.focusedViewId = result.focusedSplitViewId;

      updateViewPercentages(result.splitViews, result.focusedSplitViewId, sourceParentId);

      return true;
    },
    swapView: (firstViewId, secondViewId) => {
      if (!context.state.current.view) return false;

      const firstView = findViewById(context.state.current.view, firstViewId);
      const secondView = findViewById(context.state.current.view, secondViewId);

      if (!firstView || firstView.type !== 'item' || !secondView || secondView.type !== 'item') {
        return false;
      }

      const firstSlug = firstView.slug;
      const secondSlug = secondView.slug;

      let result = replaceSplitView(context.state.current.view, firstViewId, secondSlug);
      result = replaceSplitView(result, secondViewId, firstSlug);

      context.state.current.view = result;
      context.state.current.focusedViewId = secondViewId;

      return true;
    },
    closeSplitView: (viewId) => {
      if (!context.state.current.view) return false;

      const targetView = findViewById(context.state.current.view, viewId);
      if (!targetView) return false;

      const parentId = getParentView(context.state.current.view, viewId)?.id ?? null;

      context.state.current.view = closeSplitView(context.state.current.view, viewId);

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [viewId]: _removed, ...cleanedCurrentPercentages } = context.state.current.currentPercentages;
      context.state.current.currentPercentages = cleanedCurrentPercentages;

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [viewId]: _removedBase, ...cleanedBasePercentages } = context.state.current.basePercentages;
      context.state.current.basePercentages = cleanedBasePercentages;

      if (parentId && context.state.current.view) {
        const parent = findViewById(context.state.current.view, parentId);
        if (parent) {
          normalizeViewPercentages(parent);
        }
      }

      if (context.state.current.focusedViewId === viewId) {
        context.state.current.focusedViewId = null;
      }

      return true;
    },
    replaceSplitView: (viewId, newSlug) => {
      if (!context.state.current.view) return false;

      context.state.current.view = replaceSplitView(context.state.current.view, viewId, newSlug);
      context.state.current.focusedViewId = viewId;

      return true;
    },
  };

  setContext(key, context);

  return context;
};

export const setupViewContext = (viewItem: SplitViewItem) => {
  setContext('viewContext', viewItem);
};

export const getViewContext = () => {
  return getContext<SplitViewItem>('viewContext');
};
