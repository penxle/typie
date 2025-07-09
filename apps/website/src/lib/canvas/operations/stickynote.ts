import { DEFAULT_SIZE, MIN_SIZE } from '../const';
import { TypedStickyNote } from '../shapes/stickynote';
import { createResizeOperation } from './resize';
import type { Operation } from '../types';

export const stickynote: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const shape = new TypedStickyNote({
    x: anchor.x,
    y: anchor.y,
    width: 0,
    height: 0,
    backgroundColor: 'yellow',
    text: '',
  });

  return {
    update: (event) => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const deltaX = Math.abs(head.x - anchor.x);
      const deltaY = Math.abs(head.y - anchor.y);

      const width = Math.max(deltaX, MIN_SIZE * 10);
      const height = Math.max(deltaY, MIN_SIZE * 10);

      shape.setAttrs({ width, height });

      canvas.scene.add(shape);
      canvas.selection.nodes([shape]);

      canvas.setOperation(createResizeOperation('br'), event);
    },
    destroy: () => {
      const { width, height } = shape.attrs;
      if (!width || !height) {
        const width = DEFAULT_SIZE;
        const height = DEFAULT_SIZE;

        shape.setAttrs({ width, height });

        canvas.scene.add(shape);
        canvas.selection.nodes([shape]);

        canvas.syncManager?.addOrUpdateKonvaNode(shape);

        canvas.state.tool = 'select';
      }
    },
  };
};
