import { TypedArrow } from '../shapes/arrow';
import { defaultValues } from '../values';
import type { Operation } from '../types';

export const arrow: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const shape = new TypedArrow({
    x: anchor.x,
    y: anchor.y,
    dx: 0,
    dy: 0,
    roughness: defaultValues.roughness,
  });

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const dx = head.x - anchor.x;
      const dy = head.y - anchor.y;

      shape.setAttrs({ dx, dy });

      canvas.scene.add(shape);
    },
    destroy: () => {
      const { dx, dy } = shape.attrs;
      if (dx === 0 && dy === 0) {
        shape.destroy();
      } else {
        canvas.syncManager?.addOrUpdateKonvaNode(shape);
      }
    },
  };
};
