import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { TypedShape } from './shape';
import type { TypedRoughShapeConfig } from './types';

type TypedLineConfig = TypedRoughShapeConfig & {
  dx: number;
  dy: number;
};

export class TypedLine extends TypedShape<TypedLineConfig> {
  get effectiveRoughness() {
    const { dx, dy, roughness } = this.attrs;

    if (roughness === 'none') {
      return 0;
    }

    const max = Math.max(Math.abs(dx), Math.abs(dy));

    return clamp(max / MIN_SIZE - 1, 0.5, 2.5);
  }

  override renderView(context: Konva.Context) {
    const { dx, dy, seed } = this.attrs;

    const drawable = roughGenerator.line(0, 0, dx, dy, {
      roughness: this.effectiveRoughness,
      bowing: 0.5,
      stroke: 'black',
      strokeWidth: 4,
      seed,
    });

    renderRoughDrawable(context, drawable);
  }

  override renderHitTest(context: Konva.Context) {
    const { dx, dy } = this.attrs;

    context.strokeStyle = this.colorKey;
    context.lineWidth = 10;

    context.beginPath();
    context.moveTo(0, 0);
    context.lineTo(dx, dy);
    context.stroke();
  }

  getWidth() {
    return Math.abs(this.attrs.dx);
  }

  getHeight() {
    return Math.abs(this.attrs.dy);
  }

  override getSelfRect() {
    const { dx, dy } = this.attrs;

    return {
      x: Math.min(0, dx),
      y: Math.min(0, dy),
      width: Math.abs(dx),
      height: Math.abs(dy),
    };
  }
}

TypedLine.prototype._attrsAffectingSize = ['dx', 'dy'];
