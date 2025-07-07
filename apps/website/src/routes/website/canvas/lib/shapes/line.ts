import Konva from 'konva';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { TypedShape } from './shape';
import type { TypedRoughShapeConfig } from './types';

type TypedLineConfig = TypedRoughShapeConfig & {
  dx: number;
  dy: number;
};

export class TypedLine extends TypedShape<TypedLineConfig> {
  override renderView(context: Konva.Context) {
    const { dx, dy, roughness, seed } = this.attrs;

    const drawable = roughGenerator.line(0, 0, dx, dy, {
      roughness: roughness === 'rough' ? 2 : 0,
      bowing: 1,
      stroke: 'black',
      strokeWidth: 2,
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
