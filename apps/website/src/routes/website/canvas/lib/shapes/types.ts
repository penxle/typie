export type TypedShapeConfig = {
  x: number;
  y: number;
};

export type TypedRoughShapeConfig = TypedShapeConfig & {
  roughness: 'none' | 'rough';
  seed: number;
};

export type TypedContentfulShapeConfig = TypedRoughShapeConfig & {
  backgroundColor: string;
  backgroundStyle: 'none' | 'solid' | 'hachure';
};
