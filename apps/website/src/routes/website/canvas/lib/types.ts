import type Konva from 'konva';
import type { Canvas } from './canvas.svelte';

export type Tool = 'pan' | 'select' | 'brush' | 'rectangle' | 'ellipse' | 'line' | 'stickynote';
export type Shapes = 'brush' | 'ellipse' | 'line' | 'rectangle' | 'stickynote';

export type Pos = { x: number; y: number };

export type ResizeHandle = 'tl' | 'tr' | 'br' | 'bl' | 't' | 'r' | 'b' | 'l';

// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export type Operation = (canvas: Canvas, event?: Konva.KonvaPointerEvent) => Partial<OperationReturn> | void;
export type OperationReturn = { update: (event?: Konva.KonvaPointerEvent) => void; destroy: (event?: Konva.KonvaPointerEvent) => void };
