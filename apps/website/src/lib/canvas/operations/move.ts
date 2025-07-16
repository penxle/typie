import Konva from 'konva';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import { TypedArrow, TypedBrush, TypedEllipse, TypedLine, TypedRect, TypedStickyNote } from '../shapes';
import { getClosestGroup } from '../utils';
import type { Operation, SerializedShape, Shapes } from '../types';

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

  let initialOffsets = targets.map((node) => ({
    node,
    offsetX: node.x() - relativePos.x,
    offsetY: node.y() - relativePos.y,
  }));

  const initialPointerPos = pos;
  let hasCloned = false;

  return {
    update: () => {
      const currentPos = canvas.stage.getRelativePointerPosition();
      if (!currentPos || initialOffsets.length === 0) {
        return;
      }

      const dx = currentPos.x + initialOffsets[0].offsetX - initialPointerPos.x;
      const dy = currentPos.y + initialOffsets[0].offsetY - initialPointerPos.y;
      const distance = Math.hypot(dx, dy);
      const hasDragged = distance > 3;

      if (hasDragged && !hasCloned && event?.evt.altKey) {
        const nodes = canvas.selection.nodes();
        const newNodes: Konva.Node[] = [];

        const shapes: SerializedShape[] = nodes.map((node) => ({
          type: node.className as Shapes,
          attrs: { ...node.attrs, id: nanoid(32) },
        }));

        for (const shape of shapes) {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const attrs = shape.attrs as any;
          const node = match(shape.type)
            .with('TypedRect', () => new TypedRect(attrs))
            .with('TypedEllipse', () => new TypedEllipse(attrs))
            .with('TypedLine', () => new TypedLine(attrs))
            .with('TypedArrow', () => new TypedArrow(attrs))
            .with('TypedBrush', () => new TypedBrush(attrs))
            .with('TypedStickyNote', () => new TypedStickyNote(attrs))
            .exhaustive();

          canvas.scene.add(node);
          canvas.syncManager?.addOrUpdateKonvaNode(node);
          newNodes.push(node);
        }

        canvas.selection.nodes(newNodes);
        initialOffsets = newNodes.map((node) => ({
          node,
          offsetX: node.x() - currentPos.x,
          offsetY: node.y() - currentPos.y,
        }));

        hasCloned = true;
        canvas.scene.batchDraw();
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
