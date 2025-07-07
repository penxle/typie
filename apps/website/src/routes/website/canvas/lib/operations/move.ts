import Konva from 'konva';
import type { Operation } from '../types';

export const move: Operation = (canvas) => {
  const pos = canvas.stage.getPointerPosition();
  if (!pos) {
    return;
  }

  let shape: Konva.Node | null = canvas.scene.getIntersection(pos);
  if (!shape) {
    return;
  }

  let parent = shape.getParent();
  while (parent && !(parent instanceof Konva.Layer)) {
    if (parent instanceof Konva.Group) {
      shape = parent;
      break;
    }

    parent = parent.getParent();
  }

  canvas.selection.nodes([shape]);

  const relativePos = shape.getRelativePointerPosition();
  if (!relativePos) {
    return;
  }

  return {
    update: () => {
      const pos = canvas.stage.getRelativePointerPosition();
      if (!pos) {
        return;
      }

      shape.setAttrs({ x: pos.x - relativePos.x, y: pos.y - relativePos.y });
      canvas.selection.update();
    },
  };
};
