import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { TypedShape } from './shape';
import type { TypedRoughShapeConfig } from './types';

type TypedArrowConfig = TypedRoughShapeConfig & {
  dx: number;
  dy: number;
};

export class TypedArrow extends TypedShape<TypedArrowConfig> {
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

    const lineDrawable = roughGenerator.line(0, 0, dx, dy, {
      roughness: this.effectiveRoughness,
      bowing: 1,
      stroke: 'black',
      strokeWidth: 4,
      seed,
      disableMultiStroke: true,
      preserveVertices: true,
    });

    renderRoughDrawable(context, lineDrawable);

    const angle = Math.atan2(dy, dx);
    const arrowLength = 20;
    const arrowAngle = Math.PI / 6;

    const arrowX1 = dx - arrowLength * Math.cos(angle - arrowAngle);
    const arrowY1 = dy - arrowLength * Math.sin(angle - arrowAngle);
    const arrowX2 = dx - arrowLength * Math.cos(angle + arrowAngle);
    const arrowY2 = dy - arrowLength * Math.sin(angle + arrowAngle);

    const arrowPath = `M ${arrowX1} ${arrowY1} L ${dx} ${dy} L ${arrowX2} ${arrowY2}`;

    const arrowDrawable = roughGenerator.path(arrowPath, {
      roughness: this.effectiveRoughness * 0.5,
      bowing: 1,
      stroke: 'black',
      strokeWidth: 4,
      seed,
      disableMultiStroke: true,
      preserveVertices: true,
    });

    renderRoughDrawable(context, arrowDrawable);
  }

  override renderHitTest(context: Konva.Context) {
    const { dx, dy } = this.attrs;

    context.strokeStyle = this.colorKey;
    context.lineWidth = 10;

    context.beginPath();
    context.moveTo(0, 0);
    context.lineTo(dx, dy);
    context.stroke();

    const angle = Math.atan2(dy, dx);
    const arrowLength = 20;
    const arrowAngle = Math.PI / 6;

    context.beginPath();
    context.moveTo(dx - arrowLength * Math.cos(angle - arrowAngle), dy - arrowLength * Math.sin(angle - arrowAngle));
    context.lineTo(dx, dy);
    context.lineTo(dx - arrowLength * Math.cos(angle + arrowAngle), dy - arrowLength * Math.sin(angle + arrowAngle));
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

TypedArrow.prototype.className = 'TypedArrow';
TypedArrow.prototype._attrsAffectingSize = ['dx', 'dy'];
