import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { TypedShape } from './shape';
import type { TypedContentfulShapeConfig } from './types';

type TypedEllipseConfig = TypedContentfulShapeConfig & {
  radiusX: number;
  radiusY: number;
};

export class TypedEllipse extends TypedShape<TypedEllipseConfig> {
  get effectiveRoughness() {
    const { radiusX, radiusY, roughness } = this.attrs;

    if (roughness === 'none') {
      return 0;
    }

    const min = Math.min(radiusX * 2, radiusY * 2);

    return clamp(min / MIN_SIZE - 1, 0.5, 2.5);
  }
  override renderView(context: Konva.Context) {
    const { radiusX, radiusY, backgroundColor, backgroundStyle, seed } = this.attrs;

    const drawable = roughGenerator.ellipse(0, 0, radiusX * 2, radiusY * 2, {
      roughness: this.effectiveRoughness,
      bowing: 0.5,
      stroke: 'black',
      strokeWidth: 4,
      seed,
      fill: backgroundStyle === 'none' ? undefined : backgroundColor,
      fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
      fillWeight: 1,
      hachureGap: 8,
      curveFitting: 1,
    });

    renderRoughDrawable(context, drawable);
  }

  override renderHitTest(context: Konva.Context) {
    const { radiusX, radiusY } = this.attrs;

    context.beginPath();
    context.ellipse(0, 0, radiusX, radiusY, 0, 0, Math.PI * 2);
    context.fillStrokeShape(this);
  }

  getWidth() {
    return this.attrs.radiusX * 2;
  }

  getHeight() {
    return this.attrs.radiusY * 2;
  }
}

TypedEllipse.prototype._centroid = true;
TypedEllipse.prototype._attrsAffectingSize = ['radiusX', 'radiusY'];
