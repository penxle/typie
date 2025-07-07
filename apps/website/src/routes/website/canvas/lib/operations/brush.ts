import { TypedBrush } from '../shapes/brush';
import type { Operation } from '../types';

export const brush: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const points: [number, number][] = [[0, 0]];
  const shape = new TypedBrush({
    x: anchor.x,
    y: anchor.y,
    points: [[0, 0]],
  });

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const dx = head.x - anchor.x;
      const dy = head.y - anchor.y;

      points.push([dx, dy]);
      shape.setAttrs({ points });

      canvas.scene.add(shape);
    },
    destroy: () => {
      const { points } = shape.attrs;
      if (points.length < 2) {
        shape.destroy();
      }
    },
  };
};
