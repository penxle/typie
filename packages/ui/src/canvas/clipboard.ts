import { nanoid } from 'nanoid';
import type Konva from 'konva';
import type { SerializedShape, Shapes } from './types';

export type ClipboardData = {
  shapes: SerializedShape[];
};

export const copyShapesToClipboard = async (nodes: Konva.Node[]): Promise<void> => {
  if (nodes.length === 0) return;

  const shapes: SerializedShape[] = nodes.map((node) => ({
    type: node.className as Shapes,
    attrs: node.attrs,
  }));

  const data: ClipboardData = { shapes };

  try {
    await navigator.clipboard.writeText(JSON.stringify(data));
  } catch {
    // pass
  }
};

export const getShapesFromClipboard = async (): Promise<SerializedShape[] | null> => {
  try {
    const text = await navigator.clipboard.readText();
    const data = JSON.parse(text) as ClipboardData;

    if (data.shapes && Array.isArray(data.shapes)) {
      return data.shapes;
    }
  } catch {
    // pass
  }

  return null;
};

export const offsetShapes = (shapes: SerializedShape[], offsetX = 20, offsetY = 20): SerializedShape[] => {
  return shapes.map((shape) => ({
    ...shape,
    attrs: {
      ...shape.attrs,
      id: nanoid(32),
      x: (shape.attrs.x as number) + offsetX,
      y: (shape.attrs.y as number) + offsetY,
    },
  }));
};
