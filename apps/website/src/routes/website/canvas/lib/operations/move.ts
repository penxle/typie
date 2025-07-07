import Konva from 'konva';
import { getClosestGroup } from '../utils';
import type { Operation } from '../types';

export const move: Operation = (canvas) => {
  const pos = canvas.stage.getPointerPosition();
  if (!pos) {
    return;
  }

  let targets: Konva.Node[] = [];

  if (canvas.selection.isInsideBoundingBox(pos)) {
    targets = canvas.selection.nodes();
  } else {
    let clickedShape: Konva.Node | null = canvas.scene.getIntersection(pos);

    if (clickedShape) {
      clickedShape = getClosestGroup(clickedShape);

      if (canvas.selection.contains(clickedShape)) {
        targets = canvas.selection.nodes();
      } else {
        canvas.selection.nodes([clickedShape]);
        targets = [clickedShape];
      }
    } else {
      return;
    }
  }

  const relativePos = canvas.stage.getRelativePointerPosition();
  if (!relativePos) {
    return;
  }

  const initialOffsets = targets.map((node) => ({
    node,
    offsetX: node.x() - relativePos.x,
    offsetY: node.y() - relativePos.y,
  }));

  return {
    update: () => {
      const currentPos = canvas.stage.getRelativePointerPosition();
      if (!currentPos) {
        return;
      }

      for (const { node, offsetX, offsetY } of initialOffsets) {
        node.setAttrs({
          x: currentPos.x + offsetX,
          y: currentPos.y + offsetY,
        });
      }

      canvas.selection.update();
    },
  };
};
