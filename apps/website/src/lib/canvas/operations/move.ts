import Konva from 'konva';
import { getClosestGroup } from '../utils';
import type { Operation } from '../types';

export const move: Operation = (canvas, event) => {
  const pos = canvas.stage.getPointerPosition();
  if (!pos) {
    return;
  }

  let targets: Konva.Node[] = [];
  let clickedShape: Konva.Node | null = canvas.scene.getIntersection(pos);

  if (canvas.selection.isInsideBoundingBox(pos)) {
    if (event?.evt.shiftKey && clickedShape && canvas.selection.contains(clickedShape)) {
      canvas.selection.nodes(canvas.selection.nodes().filter((node) => node !== clickedShape));
    }
    targets = canvas.selection.nodes();
  } else {
    if (clickedShape) {
      clickedShape = getClosestGroup(clickedShape);

      if (event?.evt.shiftKey) {
        canvas.selection.nodes([...canvas.selection.nodes(), clickedShape]);
      }

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
    },
  };
};
