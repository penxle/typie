import Konva from 'konva';
import { nanoid } from 'nanoid';
import type { Shapes } from '../types';
import type { TypedShapeConfig, TypedShapeConstructorConfig } from './types';

export abstract class TypedShape<T extends TypedShapeConfig> extends Konva.Shape<T> {
  declare _type: Shapes;
  declare attrs: T;

  abstract renderView(context: Konva.Context): void;
  abstract renderHitTest(context: Konva.Context): void;

  constructor(config: TypedShapeConstructorConfig<T>) {
    const newConfig = {
      ...config,
      id: config.id || nanoid(32),
      seed: config.seed || Math.floor(Math.random() * 2_147_483_637),
    };

    super(newConfig as unknown as T);

    this.setAttr('type', this._type);
  }

  override setAttr(attr: string, val: unknown) {
    super.setAttr(attr, val);
    this.fire('attrchange', { target: this }, true);
    return this;
  }

  override setAttrs(config: Partial<T>) {
    super.setAttrs(config);
    this.fire('attrchange', { target: this }, true);
    return this;
  }

  _sceneFunc(context: Konva.Context) {
    this.renderView(context);
  }

  _hitFunc(context: Konva.Context) {
    this.renderHitTest(context);
  }
}
