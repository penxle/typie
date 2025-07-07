import Konva from 'konva';
import type { TypedShapeConfig } from './types';

export abstract class TypedShape<T extends TypedShapeConfig> extends Konva.Shape<T> {
  declare attrs: T;

  abstract renderView(context: Konva.Context): void;
  abstract renderHitTest(context: Konva.Context): void;

  override setAttrs(config: Partial<T>) {
    super.setAttrs(config);
    return this;
  }

  _sceneFunc(context: Konva.Context) {
    this.renderView(context);
  }

  _hitFunc(context: Konva.Context) {
    this.renderHitTest(context);
  }
}
