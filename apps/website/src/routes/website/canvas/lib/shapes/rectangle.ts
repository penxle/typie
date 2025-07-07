import Konva from 'konva';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { TypedShape } from './shape';
import type { TypedContentfulShapeConfig } from './types';

type TypedRectConfig = TypedContentfulShapeConfig & {
  width: number;
  height: number;
  borderRadius: 'none' | 'round';
};

export class TypedRect extends TypedShape<TypedRectConfig> {
  get effectiveBorderRadius() {
    const { width: w, height: h, borderRadius } = this.attrs;
    if (borderRadius === 'none') {
      return 0;
    }

    const side = Math.min(w, h);
    return side > 128 ? 32 : side * 0.25;
  }

  override renderView(context: Konva.Context) {
    const { width: w, height: h, roughness, backgroundColor, backgroundStyle, seed } = this.attrs;
    const r = this.effectiveBorderRadius;

    const d = `M ${r} 0 L ${w - r} 0 Q ${w} 0, ${w} ${r} L ${w} ${h - r} Q ${w} ${h}, ${w - r} ${h} L ${r} ${h} Q 0 ${h}, 0 ${h - r} L 0 ${r} Q 0 0, ${r} 0`;
    const drawable = roughGenerator.path(d, {
      roughness: roughness === 'rough' ? 2 : 0,
      bowing: 1,
      stroke: 'black',
      strokeWidth: 2,
      seed,
      fill: backgroundStyle === 'none' ? undefined : backgroundColor,
      fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
      fillWeight: 1,
      hachureGap: 8,
      preserveVertices: true,
    });

    renderRoughDrawable(context, drawable);
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
