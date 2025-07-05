import type { Canvas } from './canvas.svelte';

export type Tool = 'select' | 'freedraw' | 'rectangle' | 'ellipse' | 'line';

export type Pos = { x: number; y: number };

// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export type Operation = (canvas: Canvas, event: PointerEvent) => Partial<OperationReturn> | void;
export type OperationReturn = { update: (event?: PointerEvent) => void; destroy: (event?: PointerEvent) => void };
