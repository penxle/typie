import { GRID_SIZE } from './const';

export const gridSnap = (value: number) => Math.round(value / GRID_SIZE) * GRID_SIZE;
