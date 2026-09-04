import { createStableContext } from '@typie/ui/context/stable';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { TreeEntity } from './@selection/types';

export type TreeState = {
  entities: TreeEntity[];
  treeEntityMap: SvelteMap<string, TreeEntity>;
  dragging: boolean;
  lastSelectedEntityId?: string;
  selectedEntityIds: SvelteSet<string>;
  element?: HTMLElement;
};

const [getTreeContext, setTreeContext] = createStableContext<TreeState>('tree.TreeContext');

export { getTreeContext };

export const setupTreeContext = () => {
  const treeState = $state<TreeState>({
    entities: [],
    treeEntityMap: new SvelteMap<string, TreeEntity>(),
    dragging: false,
    lastSelectedEntityId: undefined,
    selectedEntityIds: new SvelteSet<string>(),
    element: undefined,
  });

  setTreeContext(treeState);

  return treeState;
};

export const getTreeStateEntity = (treeState: TreeState, entityId: string) => treeState.treeEntityMap.get(entityId);
