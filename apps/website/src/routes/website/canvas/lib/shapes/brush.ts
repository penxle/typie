import Konva from 'konva';
import { TypedShape } from './shape';
import type { TypedShapeConfig } from './types';

type TypedBrushConfig = TypedShapeConfig & {
  points: [number, number][];
};

export class TypedBrush extends TypedShape<TypedBrushConfig> {
  override renderView(context: Konva.Context) {
    const { points } = this.attrs;

    if (points.length < 2) {
      return;
    }

    context.strokeStyle = 'black';

    context.lineWidth = 10;
    context.lineCap = 'round';
    context.lineJoin = 'round';

    context.beginPath();
    context.moveTo(points[0][0], points[0][1]);
    for (let i = 1; i < points.length; i++) {
      context.lineTo(points[i][0], points[i][1]);
    }
    context.stroke();
  }

  override renderHitTest(context: Konva.Context) {
    const { points } = this.attrs;

    if (points.length < 2) {
      return;
    }

    context.strokeStyle = this.colorKey;

    context.lineWidth = 10;
    context.lineCap = 'round';
    context.lineJoin = 'round';

    context.beginPath();
    context.moveTo(points[0][0], points[0][1]);
    for (let i = 1; i < points.length; i++) {
      context.lineTo(points[i][0], points[i][1]);
    }
    context.stroke();
  }

  getWidth() {
    const { points } = this.attrs;

    if (points.length === 0) {
      return 0;
    }

    const xCoords = points.map((p) => p[0]);
    const minX = Math.min(...xCoords);
    const maxX = Math.max(...xCoords);

    return Math.abs(maxX - minX);
  }

  getHeight() {
    const { points } = this.attrs;

    if (points.length === 0) {
      return 0;
    }

    const yCoords = points.map((p) => p[1]);
    const minY = Math.min(...yCoords);
    const maxY = Math.max(...yCoords);

    return Math.abs(maxY - minY);
  }

  override getSelfRect() {
    const { points } = this.attrs;

    if (points.length === 0) {
      return { x: 0, y: 0, width: 0, height: 0 };
    }

    const xCoords = points.map((p) => p[0]);
    const yCoords = points.map((p) => p[1]);
    const minX = Math.min(...xCoords);
    const maxX = Math.max(...xCoords);
    const minY = Math.min(...yCoords);
    const maxY = Math.max(...yCoords);

    return {
      x: minX,
      y: minY,
      width: Math.abs(maxX - minX),
      height: Math.abs(maxY - minY),
    };
  }
}

TypedBrush.prototype._attrsAffectingSize = ['points'];
