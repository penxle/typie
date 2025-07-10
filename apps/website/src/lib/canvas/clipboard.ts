import { nanoid } from 'nanoid';
import type Konva from 'konva';
import type { Shapes } from './types';

export type ClipboardShape = {
  type: Shapes;
  attrs: Record<string, unknown>;
};

export type ClipboardData = {
  shapes: ClipboardShape[];
};

export const copyShapesToClipboard = async (nodes: Konva.Node[]): Promise<void> => {
  if (nodes.length === 0) return;

  const shapes: ClipboardShape[] = nodes.map((node) => ({
    type: node.attrs.type as Shapes,
    attrs: { ...node.attrs },
  }));

  const data: ClipboardData = { shapes };

  try {
    await navigator.clipboard.writeText(JSON.stringify(data));
  } catch {
    // pass
  }
};

export const getShapesFromClipboard = async (): Promise<ClipboardShape[] | null> => {
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

export const offsetShapes = (shapes: ClipboardShape[], offsetX = 20, offsetY = 20): ClipboardShape[] => {
  return shapes.map((shape) => ({
    ...shape,
    attrs: {
      ...shape.attrs,
      id: nanoid(),
      x: (shape.attrs.x as number) + offsetX,
      y: (shape.attrs.y as number) + offsetY,
    },
  }));
};
