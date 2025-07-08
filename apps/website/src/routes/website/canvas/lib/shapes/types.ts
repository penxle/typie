import type { Shapes } from '../types';
import type { BackgroundStyle, Roughness } from '../values';

export type TypedShapeConfig = {
  id: string;
  type: Shapes;
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

export type TypedShapeConstructorConfig<T extends TypedShapeConfig> = Omit<T, 'id' | 'type' | 'seed'> & { id?: string; seed?: number };
