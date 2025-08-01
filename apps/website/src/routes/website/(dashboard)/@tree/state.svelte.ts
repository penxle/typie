import { getContext, setContext } from 'svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { TreeEntity } from './@selection/types';

type TreeState = {
  entities: TreeEntity[];
  entityMap: SvelteMap<string, TreeEntity>;
  lastSelectedEntityId?: string;
  selectedEntityIds: SvelteSet<string>;
  element?: HTMLElement;
};

const key: unique symbol = Symbol('TreeContext');

export const getTreeContext = () => {
  return getContext<TreeState>(key);
};

export const setupTreeContext = () => {
  const treeState = $state<TreeState>({
    entities: [],
    entityMap: new SvelteMap<string, TreeEntity>(),
    lastSelectedEntityId: undefined,
    selectedEntityIds: new SvelteSet<string>(),
    element: undefined,
  });

  setContext(key, treeState);

  return treeState;
};
