import type { Node } from '@tiptap/pm/model';
import type { Selection } from '@tiptap/pm/state';

export type NodeWithDepth = {
  node: Node;
  depth: number;
  pos: number;
};

export function traverseNodesUpward(selection: Selection, callback: (nodeInfo: NodeWithDepth) => boolean | undefined): void {
  const { $from } = selection;

  for (let depth = $from.depth; depth >= 0; depth--) {
    const node = $from.node(depth);
    const pos = $from.before(depth);

    if (callback({ node, depth, pos })) {
      break;
    }
  }
}

export function findNodeUpward(selection: Selection, predicate: (nodeInfo: NodeWithDepth) => boolean): NodeWithDepth | null {
  let result: NodeWithDepth | null = null;

  traverseNodesUpward(selection, (nodeInfo) => {
    if (predicate(nodeInfo)) {
      result = nodeInfo;
      return true;
    }
  });

  return result;
}
