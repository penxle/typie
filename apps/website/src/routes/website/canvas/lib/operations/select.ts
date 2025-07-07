import Konva from 'konva';
import type { Operation } from '../types';

export const select: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  canvas.selection.nodes([]);

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const x = Math.min(anchor.x, head.x);
      const y = Math.min(anchor.y, head.y);
      const width = Math.abs(head.x - anchor.x);
      const height = Math.abs(head.y - anchor.y);

      canvas.selection.showIndicator(x, y, width, height);

      const clientRect = canvas.selection.getIndicatorClientRect();
      const nodes = canvas.scene.children.filter((child) => {
        const childRect = child.getClientRect();
        return Konva.Util.haveIntersection(clientRect, childRect);
      });

      canvas.selection.nodes(nodes);
    },
    destroy: () => {
      canvas.selection.hideIndicator();
    },
  };
};
