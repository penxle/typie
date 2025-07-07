import type { Operation } from '../types';

export const pan: Operation = (canvas) => {
  let last = canvas.stage.getPointerPosition();
  if (!last) {
    return;
  }

  return {
    update: () => {
      const current = canvas.stage.getPointerPosition();
      if (!last || !current) {
        return;
      }

      const deltaX = current.x - last.x;
      const deltaY = current.y - last.y;
      last = current;

      if (deltaX !== 0 || deltaY !== 0) {
        canvas.moveBy(deltaX, deltaY);
      }
    },
  };
};
