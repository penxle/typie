import { DEFAULT_SIZE, MIN_SIZE } from '../const';
import { TypedRect } from '../shapes/rectangle';
import { defaultValues } from '../values';
import { createResizeOperation } from './resize';
import type { Operation } from '../types';

export const rectangle: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const shape = new TypedRect({
    x: anchor.x,
    y: anchor.y,
    width: 0,
    height: 0,
    borderRadius: defaultValues.borderRadius,
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

      const width = Math.max(deltaX, MIN_SIZE);
      const height = Math.max(deltaY, MIN_SIZE);

      shape.setAttrs({ width, height });

      canvas.scene.add(shape);
      canvas.selection.nodes([shape]);

      shape.startEditing();

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

        shape.startEditing();

        canvas.syncManager?.addOrUpdateKonvaNode(shape);

        canvas.state.tool = 'select';
      }
    },
  };
};
