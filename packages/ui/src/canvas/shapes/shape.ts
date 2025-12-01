import Konva from 'konva';
import { nanoid } from 'nanoid';
import type { TypedShapeConfig, TypedShapeConstructorConfig } from './types';

export abstract class TypedShape<T extends TypedShapeConfig> extends Konva.Shape<T> {
  declare attrs: T;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  declare hitFunc: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  declare sceneFunc: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  declare on: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  declare setAttr: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  declare setAttrs: any;

  abstract renderView(context: Konva.Context): void;
  abstract renderHitTest(context: Konva.Context): void;

  constructor(config: TypedShapeConstructorConfig<T>) {
    const newConfig = {
      ...config,
      id: config.id || nanoid(32),
      seed: config.seed || Math.floor(Math.random() * 2_147_483_637),
    };

    super(newConfig as unknown as T);

    const originalSetAttr = this.setAttr.bind(this);
    this.setAttr = (attr: string, val: unknown) => {
      originalSetAttr(attr, val);
      this.fire('attrchange', { target: this }, true);
      return this;
    };

    const originalSetAttrs = this.setAttrs.bind(this);
    this.setAttrs = (config?: Partial<T>) => {
      originalSetAttrs(config);
      this.fire('attrchange', { target: this }, true);
      return this;
    };
  }

  _sceneFunc(context: Konva.Context) {
    this.renderView(context);
  }

  _hitFunc(context: Konva.Context) {
    this.renderHitTest(context);
  }
}
