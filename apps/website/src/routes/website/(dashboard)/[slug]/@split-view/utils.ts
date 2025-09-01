import { nanoid } from 'nanoid';
import type { Ref } from '@typie/ui/utils';
import type { SplitView, SplitViewState } from './context.svelte';

export const VIEW_MIN_SIZE = 210;
export const RESIZER_SIZE = 4;
export const BUFFER_SIZE = 4; // NOTE: 어쩐지 없으면 스크롤 생긴다

export const getMinSizeForView = (view: SplitView, parentDirection: 'horizontal' | 'vertical'): number => {
  if (view.type === 'item') {
    return VIEW_MIN_SIZE;
  }

  // NOTE: 부모와 같은 방향: 자식들의 최소 크기 합산 + Resizer 크기들
  if (view.direction === parentDirection) {
    const childrenMinSize = view.children.reduce((sum, child) => sum + getMinSizeForView(child, parentDirection), 0);
    const resizerCount = Math.max(0, view.children.length - 1);
    return childrenMinSize + resizerCount * (RESIZER_SIZE + BUFFER_SIZE);
  } else {
    // NOTE: 부모와 다른 방향: 자식들 중 최대 크기
    return Math.max(...view.children.map((child) => getMinSizeForView(child, parentDirection)));
  }
};

export const collectSlug = (splitViews: SplitView | null): string[] => {
  if (!splitViews) {
    return [];
  }

  if (splitViews.type === 'item') {
    return [splitViews.slug];
  }
  return splitViews.children.flatMap((child) => collectSlug(child));
};

export const addSplitView = (
  splitViews: SplitView,
  slug: string,
  direction: 'horizontal' | 'vertical',
): { splitViews: SplitView; focusedSplitViewId: string } => {
  const newSplitViewId = nanoid();

  if (splitViews.type === 'container' && splitViews.direction === direction) {
    return {
      splitViews: {
        ...splitViews,
        children: [...splitViews.children, { id: newSplitViewId, slug, type: 'item' }],
      },
      focusedSplitViewId: newSplitViewId,
    };
  }

  return {
    splitViews: {
      id: nanoid(),
      type: 'container',
      direction,
      children: [splitViews, { id: newSplitViewId, slug, type: 'item' }],
    },
    focusedSplitViewId: newSplitViewId,
  };
};

export const closeSplitView = (splitViews: SplitView, splitViewId: string): SplitView | null => {
  if (splitViews.type === 'item' && splitViews.id === splitViewId) {
    return null;
  }

  if (splitViews.type === 'container' && splitViews.children) {
    splitViews.children = splitViews.children.map((child) => closeSplitView(child, splitViewId)).filter((child) => child !== null);

    const remainingChildren = splitViews.children.filter((child) => child !== null);

    if (remainingChildren.length === 1) {
      return {
        ...remainingChildren[0],
        id: splitViews.id,
      };
    }

    if (remainingChildren.length === 0) {
      return null;
    }
  }

  return splitViews;
};

export const replaceSplitView = (view: SplitView, id: string, newSlug: string): SplitView => {
  if (view.type === 'item') {
    return view.id === id ? { ...view, slug: newSlug } : view;
  }

  let changed = false;
  const children = view.children.map((child) => {
    const next = replaceSplitView(child, id, newSlug);
    if (next !== child) changed = true;
    return next;
  });
  return changed ? { ...view, children } : view;
};

export const findViewIdBySlug = (splitViews: SplitView, slug: string): string | null => {
  if (splitViews.type === 'item' && splitViews.slug === slug) {
    return splitViews.id;
  }

  if (splitViews.type === 'container' && splitViews.children) {
    return splitViews.children.map((child) => findViewIdBySlug(child, slug)).find((id) => id !== null) ?? null;
  }

  return null;
};

export const getParentView = (splitViews: SplitView, viewId: string): SplitView | null => {
  if (splitViews.type === 'container' && splitViews.children) {
    if (splitViews.children.some((child) => child.id === viewId)) {
      return splitViews;
    }

    for (const child of splitViews.children) {
      if (child.type === 'container') {
        const parent = getParentView(child, viewId);
        if (parent) {
          return parent;
        }
      }
    }
  }

  return null;
};

export const calculateViewPercentages = (
  parentView: SplitView,
  newViewId: string,
  currentPercentages: Record<string, number>,
): Record<string, number> => {
  if (parentView.type !== 'container') {
    return currentPercentages;
  }

  const existingChildren = parentView.children.filter((child) => child.id !== newViewId);
  const newChildCount = parentView.children.length;

  const newViewPercentage = 100 / newChildCount;
  const remainingPercentage = 100 - newViewPercentage;

  let currentTotal = 0;
  existingChildren.forEach((child) => {
    currentTotal += currentPercentages[child.id] || 100 / existingChildren.length;
  });

  const scaleFactor = currentTotal > 0 ? remainingPercentage / currentTotal : 0;

  const newPercentages: Record<string, number> = {};

  parentView.children.forEach((child) => {
    if (child.id === newViewId) {
      newPercentages[child.id] = newViewPercentage;
    } else {
      const currentPercentage = currentPercentages[child.id] || 100 / existingChildren.length;
      newPercentages[child.id] = currentPercentage * scaleFactor;
    }
  });

  return newPercentages;
};

export const addSplitViewToState = (state: Ref<SplitViewState>, slug: string, direction: 'horizontal' | 'vertical'): void => {
  if (!state.current.view) return;

  const { splitViews, focusedSplitViewId } = addSplitView(state.current.view, slug, direction);
  state.current.view = splitViews;
  state.current.focusedViewId = focusedSplitViewId;

  const parentView = getParentView(splitViews, focusedSplitViewId);
  if (parentView && parentView.type === 'container') {
    const newPercentages = calculateViewPercentages(parentView, focusedSplitViewId, state.current.currentPercentages);

    state.current.currentPercentages = {
      ...state.current.currentPercentages,
      ...newPercentages,
    };

    state.current.basePercentages = {
      ...state.current.basePercentages,
      [focusedSplitViewId]: newPercentages[focusedSplitViewId],
    };
  }
};

export const addViewToSplitView = (
  splitViews: SplitView,
  targetViewId: string,
  newSlug: string,
  direction: 'horizontal' | 'vertical',
  position: 'before' | 'after',
): { splitViews: SplitView; focusedSplitViewId: string } => {
  const newViewId = nanoid();

  const replaceWithContainer = (view: SplitView): SplitView => {
    if (view.type === 'item' && view.id === targetViewId) {
      const children =
        position === 'before'
          ? [
              { id: newViewId, slug: newSlug, type: 'item' as const },
              { ...view, id: nanoid() },
            ]
          : [
              { ...view, id: nanoid() },
              { id: newViewId, slug: newSlug, type: 'item' as const },
            ];

      return {
        id: view.id,
        type: 'container',
        direction,
        children,
      };
    }

    if (view.type === 'container') {
      const childIndex = view.children.findIndex((child) => child.id === targetViewId);

      if (childIndex !== -1 && view.direction === direction) {
        const newChild = { id: newViewId, slug: newSlug, type: 'item' as const };
        const newChildren = [...view.children];

        if (position === 'before') {
          newChildren.splice(childIndex, 0, newChild);
        } else {
          newChildren.splice(childIndex + 1, 0, newChild);
        }

        return {
          ...view,
          children: newChildren,
        };
      }

      return {
        ...view,
        children: view.children.map((child) => replaceWithContainer(child)),
      };
    }

    return view;
  };

  return {
    splitViews: replaceWithContainer(splitViews),
    focusedSplitViewId: newViewId,
  };
};
