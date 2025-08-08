import Konva from 'konva';
import { TypedArrow } from './shapes/arrow';
import { TypedLine } from './shapes/line';

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

export const isSingleLineLike = (nodes: Konva.Node[]): nodes is [TypedLine | TypedArrow] => {
  return nodes.length === 1 && (nodes[0] instanceof TypedLine || nodes[0] instanceof TypedArrow);
};
