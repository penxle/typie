import type { BackgroundStyle, Roughness } from '../values';

export type TypedShapeConfig = {
  x: number;
  y: number;
};

export type TypedRoughShapeConfig = TypedShapeConfig & {
  roughness: Roughness;
  seed: number;
};

export type TypedContentfulShapeConfig = TypedRoughShapeConfig & {
  backgroundColor: string;
  backgroundStyle: BackgroundStyle;
};
