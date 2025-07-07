import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { values } from '../values';
import { TypedShape } from './shape';
import type { TypedContentfulShapeConfig } from './types';

type TypedRectConfig = TypedContentfulShapeConfig & {
  width: number;
  height: number;
  borderRadius: 'none' | 'round';
};

export class TypedRect extends TypedShape<TypedRectConfig> {
  get effectiveBorderRadius() {
    const { width, height, borderRadius } = this.attrs;

    if (borderRadius === 'none') {
      return 0;
    }

    const min = Math.min(width, height);
    return Math.min(min * 0.25, 50);
  }

  get effectiveRoughness() {
    const { width, height, roughness } = this.attrs;

    if (roughness === 'none') {
      return 0;
    }

    const min = Math.min(width, height);

    return clamp(min / MIN_SIZE - 1, 0.5, 2.5);
  }

  override renderView(context: Konva.Context) {
    const { width: w, height: h, backgroundColor, backgroundStyle, seed } = this.attrs;
    const r = this.effectiveBorderRadius;

    const bgColorHex = values.backgroundColor.find((c) => c.value === backgroundColor)?.hex;

    if (r === 0) {
      const drawable = roughGenerator.rectangle(0, 0, w, h, {
        roughness: this.effectiveRoughness,
        bowing: 1,
        stroke: 'black',
        strokeWidth: 4,
        seed,
        fill: backgroundStyle === 'none' ? undefined : bgColorHex,
        fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
        fillWeight: 1,
        hachureGap: 8,
      });

      renderRoughDrawable(context, drawable);
    } else {
      const d = `M ${r} 0 L ${w - r} 0 Q ${w} 0, ${w} ${r} L ${w} ${h - r} Q ${w} ${h}, ${w - r} ${h} L ${r} ${h} Q 0 ${h}, 0 ${h - r} L 0 ${r} Q 0 0, ${r} 0`;
      const drawable = roughGenerator.path(d, {
        roughness: this.effectiveRoughness,
        bowing: 1,
        stroke: 'black',
        strokeWidth: 4,
        seed,
        fill: backgroundStyle === 'none' ? undefined : bgColorHex,
        fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
        fillWeight: 1,
        hachureGap: 8,
        preserveVertices: true,
      });

      renderRoughDrawable(context, drawable);
    }
  }

  override renderHitTest(context: Konva.Context) {
    const { width: w, height: h } = this.attrs;
    const r = this.effectiveBorderRadius;

    context.beginPath();
    context.moveTo(r, 0);
    context.lineTo(w - r, 0);
    context.quadraticCurveTo(w, 0, w, r);
    context.lineTo(w, h - r);
    context.quadraticCurveTo(w, h, w - r, h);
    context.lineTo(r, h);
    context.quadraticCurveTo(0, h, 0, h - r);
    context.lineTo(0, r);
    context.quadraticCurveTo(0, 0, r, 0);
    context.fillStrokeShape(this);
  }
}
