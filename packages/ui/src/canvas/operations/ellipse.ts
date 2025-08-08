import { DEFAULT_SIZE, MIN_SIZE } from '../const';
import { TypedEllipse } from '../shapes/ellipse';
import { defaultValues } from '../values';
import { createResizeOperation } from './resize';
import type { Operation } from '../types';

export const ellipse: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const shape = new TypedEllipse({
    x: anchor.x,
    y: anchor.y,
    radiusX: 0,
    radiusY: 0,
    roughness: defaultValues.roughness,
    backgroundColor: defaultValues.backgroundColor,
    backgroundStyle: defaultValues.backgroundStyle,
    text: '',
    fontSize: defaultValues.fontSize,
    fontFamily: defaultValues.fontFamily,
    textAlign: 'center',
  });

  return {
    update: (event) => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const deltaX = Math.abs(head.x - anchor.x);
      const deltaY = Math.abs(head.y - anchor.y);

      const radiusX = Math.max(deltaX, MIN_SIZE) / 2;
      const radiusY = Math.max(deltaY, MIN_SIZE) / 2;

      const x = Math.min(anchor.x, head.x) + radiusX;
      const y = Math.min(anchor.y, head.y) + radiusY;

      shape.setAttrs({ x, y, radiusX, radiusY });

      canvas.scene.add(shape);
      canvas.selection.nodes([shape]);

      shape.startEditing();

      canvas.setOperation(createResizeOperation('br'), event);
    },
    destroy: () => {
      const { radiusX, radiusY } = shape.attrs;
      if (!radiusX || !radiusY) {
        const radiusX = DEFAULT_SIZE / 2;
        const radiusY = DEFAULT_SIZE / 2;

        const x = anchor.x + radiusX;
        const y = anchor.y + radiusY;

        shape.setAttrs({ x, y, radiusX, radiusY });

        canvas.scene.add(shape);
        canvas.selection.nodes([shape]);

        shape.startEditing();

        canvas.syncManager?.addOrUpdateKonvaNode(shape);

        canvas.state.tool = 'select';
      }
    },
  };
};
