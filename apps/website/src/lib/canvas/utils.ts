import Konva from 'konva';

export const getClosestGroup = (node: Konva.Node) => {
  if (node instanceof Konva.Group) {
    return node;
  }

  let parent = node.getParent();
  while (parent && !(parent instanceof Konva.Layer)) {
    if (parent instanceof Konva.Group) {
      return parent;
    }
    parent = parent.getParent();
  }

  return node;
};
