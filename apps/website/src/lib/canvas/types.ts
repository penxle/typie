import type Konva from 'konva';
import type { Canvas } from './class.svelte';

export type Tool = 'pan' | 'select' | 'brush' | 'rectangle' | 'ellipse' | 'line' | 'arrow' | 'stickynote';
export type Shapes = 'TypedArrow' | 'TypedBrush' | 'TypedEllipse' | 'TypedLine' | 'TypedRect' | 'TypedStickyNote';

export type SerializedShape = {
  type: Shapes;
  attrs: Record<string, unknown>;
};

export type Pos = { x: number; y: number };

export type ResizeRectHandle = 'tl' | 'tr' | 'br' | 'bl' | 't' | 'r' | 'b' | 'l';
export type ResizeLineHandle = 'start' | 'end';
export type ResizeHandle = ResizeRectHandle | ResizeLineHandle;

// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export type Operation = (canvas: Canvas, event?: Konva.KonvaPointerEvent) => Partial<OperationReturn> | void;
export type OperationReturn = { update: (event?: Konva.KonvaPointerEvent) => void; destroy: (event?: Konva.KonvaPointerEvent) => void };
