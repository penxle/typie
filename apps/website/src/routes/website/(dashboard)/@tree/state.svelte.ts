import { createStableContext } from '@typie/ui/context/stable';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { TreeEntity } from './@selection/types';

export type TreeState = {
  entities: TreeEntity[];
  entityMap: SvelteMap<string, TreeEntity>;
  lastSelectedEntityId?: string;
  selectedEntityIds: SvelteSet<string>;
  element?: HTMLElement;
};

const [getTreeContext, setTreeContext] = createStableContext<TreeState>('tree.TreeContext');

export { getTreeContext };

export const setupTreeContext = () => {
  const treeState = $state<TreeState>({
    entities: [],
    entityMap: new SvelteMap<string, TreeEntity>(),
    lastSelectedEntityId: undefined,
    selectedEntityIds: new SvelteSet<string>(),
    element: undefined,
  });

  setTreeContext(treeState);

  return treeState;
};
