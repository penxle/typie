import { nanoid } from 'nanoid';
import type { SplitView } from './context.svelte';

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
        id: nanoid(),
      };
    }

    if (remainingChildren.length === 0) {
      return null;
    }
  }

  return splitViews;
};

export const replaceSplitView = (splitViews: SplitView, id: string, newSlug: string, newId: string): SplitView => {
  if (splitViews.type === 'item' && splitViews.id === id) {
    return { ...splitViews, slug: newSlug, id: newId };
  }

  if (splitViews.type === 'container' && splitViews.children) {
    splitViews.children = splitViews.children.map((child) => replaceSplitView(child, id, newSlug, newId));
  }

  return splitViews;
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
