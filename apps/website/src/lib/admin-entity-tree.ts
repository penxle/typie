export type EntityTreeInput = {
  id: string;
  parent?: { id: string } | null;
};

export type EntityTreeNode<T> = {
  entity: T;
  depth: number;
  children: EntityTreeNode<T>[];
};

export const buildEntityTree = <T extends EntityTreeInput>(entities: T[]): EntityTreeNode<T>[] => {
  const nodes = new Map<string, EntityTreeNode<T>>();

  for (const entity of entities) {
    nodes.set(entity.id, { entity, depth: 0, children: [] });
  }

  const roots: EntityTreeNode<T>[] = [];

  for (const entity of entities) {
    const node = nodes.get(entity.id);
    if (!node) {
      continue;
    }

    const parentId = entity.parent?.id;
    const parent = parentId ? nodes.get(parentId) : undefined;

    if (parent) {
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  const assignDepth = (node: EntityTreeNode<T>, depth: number) => {
    node.depth = depth;
    for (const child of node.children) {
      assignDepth(child, depth + 1);
    }
  };

  for (const root of roots) {
    assignDepth(root, 0);
  }

  return roots;
};
