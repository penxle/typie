import { DEFAULT_SIZE, MIN_SIZE } from '../const';
import * as ops from '../operations';
import { TypedStickyNote } from '../shapes/stickynote';
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
    roughness: 'rough',
    backgroundColor: '#fef3c7',
    backgroundStyle: 'solid',
    seed: Math.random() * 2_147_483_637,
    text: '',
    fontSize: 16,
    fontFamily: 'sans-serif',
  });

  return {
    update: (event) => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const deltaX = Math.abs(head.x - anchor.x);
      const deltaY = Math.abs(head.y - anchor.y);

      const width = Math.max(deltaX, MIN_SIZE);
      const height = Math.max(deltaY, MIN_SIZE);

      shape.setAttrs({ width, height });

      canvas.scene.add(shape);
      canvas.selection.nodes([shape]);

      canvas.setOperation(ops.createResizeOperation('br'), event);
    },
    destroy: () => {
      const { width, height } = shape.attrs;
      if (!width || !height) {
        const width = DEFAULT_SIZE;
        const height = DEFAULT_SIZE;

        shape.setAttrs({ width, height });

        canvas.scene.add(shape);
        canvas.selection.nodes([shape]);

        canvas.state.tool = 'select';
      }
    },
  };
};
