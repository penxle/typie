import { LocalStore } from '@typie/ui/state';
import { getContext, setContext } from 'svelte';

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

type SplitViewContext = {
  state: LocalStore<SplitViewState>;
};

export const getSplitViewContext = () => {
  return getContext<SplitViewContext>(key);
};

export const setupSplitViewContext = (userId: string) => {
  const context: SplitViewContext = {
    state: new LocalStore<SplitViewState>(`typie:splitview:${userId}`, {
      view: null,
      enabled: false,
      focusedViewId: null,
      basePercentages: {},
      currentPercentages: {},
    }),
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
