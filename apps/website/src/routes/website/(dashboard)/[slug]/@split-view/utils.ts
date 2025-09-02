import { nanoid } from 'nanoid';
import type { SplitView } from './context.svelte';

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

export const closeSplitView = (splitViews: SplitView, splitViewId: string): SplitView | null => {
  if (splitViews.type === 'item' && splitViews.id === splitViewId) {
    return null;
  }

  if (splitViews.type === 'container' && splitViews.children) {
    const remainingChildren = splitViews.children
      .map((child) => closeSplitView(child, splitViewId))
      .filter((child): child is SplitView => child !== null);

    const newView = {
      ...splitViews,
      children: remainingChildren,
    };
    return flattenSplitView(newView);
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

export const findViewById = (splitViews: SplitView, viewId: string): SplitView | null => {
  if (splitViews.id === viewId) {
    return splitViews;
  }

  if (splitViews.type === 'container' && splitViews.children) {
    for (const child of splitViews.children) {
      const found = findViewById(child, viewId);
      if (found) {
        return found;
      }
    }
  }

  return null;
};

const flattenSplitView = (view: SplitView): SplitView | null => {
  if (view.type === 'item') {
    return view;
  }

  // container인 경우 자식들을 재귀적으로 flatten
  const flattenedChildren = view.children.map((child) => flattenSplitView(child)).filter((child): child is SplitView => child !== null);

  if (flattenedChildren.length === 0) {
    return null;
  }

  // NOTE: 자식이 하나만 남으면 승격
  if (flattenedChildren.length === 1) {
    return {
      ...flattenedChildren[0],
      id: view.id,
    };
  }

  // NOTE: 방향이 같은 자식 container들을 부모 레벨로 병합
  const mergedChildren = flattenedChildren.flatMap((child) => {
    if (child.type === 'container' && child.direction === view.direction) {
      return child.children;
    }
    return [child];
  });

  return {
    ...view,
    children: mergedChildren,
  };
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

// NOTE: 새 뷰가 추가될 때 새 뷰에 균등한 공간 할당하고, 기존 뷰는 남은 공간을 기존 비율대로 나눔
export const calculateViewPercentagesForNewView = (
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

// NOTE: 뷰가 삭제되면 남은 공간을 기존 비율대로 나눠서 총합을 100%로 맞춤
export const normalizeContainerPercentages = (parentView: SplitView, current: Record<string, number>): Record<string, number> => {
  if (parentView.type !== 'container') return {};
  const ids = parentView.children.map((c) => c.id);
  const total = ids.reduce((s, id) => s + (current[id] ?? 0), 0);
  if (!total) {
    const equal = 100 / ids.length;
    return ids.reduce<Record<string, number>>((acc, id) => ((acc[id] = equal), acc), {});
  }
  const scale = 100 / total;
  return ids.reduce<Record<string, number>>((acc, id) => ((acc[id] = (current[id] ?? 0) * scale), acc), {});
};

export const addViewAtRoot = (
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

export const moveOrAddView = (
  splitViews: SplitView,
  source:
    | {
        viewId: string;
        delete: boolean;
      } // move
    | { slug: string }, // add
  target: {
    viewId: string;
    direction: 'horizontal' | 'vertical';
    position: 'before' | 'after';
  },
): { splitViews: SplitView; focusedSplitViewId: string } | null => {
  const isMove = 'viewId' in source;
  const willDeleteSource = isMove && source.delete;
  const newViewId = willDeleteSource ? source.viewId : nanoid();
  const { viewId: targetViewId, direction, position } = target;

  // NOTE: source 처리: viewId가 있으면 기존 뷰 이동, slug가 있으면 새 뷰 추가
  let sourceViewId: string | null = null;
  let sourceSlug: string;

  if (isMove) {
    sourceViewId = source.viewId;
    const sourceView = findViewById(splitViews, sourceViewId);
    if (!sourceView || sourceView.type !== 'item') {
      return null;
    }
    sourceSlug = sourceView.slug;
  } else {
    sourceSlug = source.slug;
  }

  // NOTE: 루트가 단일 item이고 그것이 타겟인 경우 바로 처리
  if (!sourceViewId && splitViews.type === 'item' && splitViews.id === targetViewId) {
    const newView = { id: newViewId, slug: sourceSlug, type: 'item' as const };
    const targetCopy = { ...splitViews, id: nanoid() };

    const children = position === 'before' ? [newView, targetCopy] : [targetCopy, newView];

    return {
      splitViews: {
        id: splitViews.id,
        type: 'container',
        direction,
        children,
      },
      focusedSplitViewId: newViewId,
    };
  }

  const processView = (view: SplitView): SplitView | null => {
    if (view.type === 'item') {
      if (sourceViewId && view.id === sourceViewId && willDeleteSource) {
        return null;
      }

      // NOTE: 타겟 뷰는 새 뷰와 함께 container로 변환
      if (view.id === targetViewId) {
        const newView = { id: newViewId, slug: sourceSlug, type: 'item' as const };
        const targetCopy = { ...view, id: nanoid() };

        const children = position === 'before' ? [newView, targetCopy] : [targetCopy, newView];

        return {
          id: view.id,
          type: 'container',
          direction,
          children,
        };
      }

      return view;
    }

    const processedChildren = view.children.map((child) => processView(child)).filter((child): child is SplitView => child !== null);

    return { ...view, children: processedChildren };
  };

  const result = processView(splitViews);

  if (!result) {
    return null;
  }

  const flattened = flattenSplitView(result);

  if (!flattened) {
    return null;
  }

  return {
    splitViews: flattened,
    focusedSplitViewId: newViewId,
  };
};
