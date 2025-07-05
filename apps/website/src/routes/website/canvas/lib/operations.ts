import Konva from 'konva';
import { getStroke } from 'perfect-freehand';
import rough from 'roughjs';
import type { Operation } from './types';

const generator = rough.generator();

export const select: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const rect = new Konva.Rect({
    stroke: 'rgba(0, 135, 255, 1)',
    strokeWidth: 0.5,
    fill: 'rgba(0, 135, 255, 0.1)',
    strokeScaleEnabled: false,
  });

  canvas.scene.add(rect);

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const x = Math.min(anchor.x, head.x);
      const y = Math.min(anchor.y, head.y);
      const width = Math.abs(head.x - anchor.x);
      const height = Math.abs(head.y - anchor.y);

      rect.setAttrs({ x, y, width, height });
    },
    destroy: () => {
      const clientRect = rect.getClientRect();
      const nodes = canvas.scene.children.filter((child) => {
        if (child === rect || child === canvas.tf) {
          return false;
        }

        const childRect = child.getClientRect();
        return Konva.Util.haveIntersection(clientRect, childRect);
      });

      canvas.tf.nodes(nodes);

      rect.destroy();
    },
  };
};

export const move: Operation = (canvas) => {
  const pos = canvas.stage.getPointerPosition();
  if (!pos) {
    return;
  }

  const shape = canvas.scene.getIntersection(pos);
  if (!shape) {
    return;
  }

  canvas.tf.nodes([shape]);

  const relativePos = shape.getRelativePointerPosition();
  if (!relativePos) {
    return;
  }

  return {
    update: () => {
      const pos = canvas.stage.getRelativePointerPosition();
      if (!pos) {
        return;
      }

      shape.setAttrs({ x: pos.x - relativePos.x, y: pos.y - relativePos.y });
    },
  };
};

export const pan: Operation = (canvas) => {
  let last = canvas.stage.getPointerPosition();
  if (!last) {
    return;
  }

  return {
    update: () => {
      const current = canvas.stage.getPointerPosition();
      if (!last || !current) {
        return;
      }

      const deltaX = current.x - last.x;
      const deltaY = current.y - last.y;
      last = current;

      if (deltaX !== 0 || deltaY !== 0) {
        canvas.moveBy(deltaX, deltaY);
      }
    },
  };
};

export const freedraw: Operation = (canvas) => {
  const points: [number, number][] = [];
  const path = new Konva.Path({
    data: '',
    fill: 'black',
  });

  canvas.scene.add(path);

  return {
    update: () => {
      const pos = canvas.stage.getRelativePointerPosition();
      if (!pos) {
        return;
      }

      points.push([pos.x, pos.y]);

      const stroke = getStroke(points, {
        size: 4,
        thinning: 0.5,
        smoothing: 0.5,
        streamline: 0.5,
      });

      if (stroke.length < 4) {
        return;
      }

      const a = (a: number, b: number) => ((a + b) / 2).toFixed(2);
      const f = ([x, y]: number[]) => `${x.toFixed(2)},${y.toFixed(2)}`;

      const [p0, p1, p2] = stroke;
      const start = `M${f(p0)} Q${f(p1)} ${a(p1[0], p2[0])},${a(p1[1], p2[1])} T`;
      const middle = stroke
        .slice(2, -1)
        .map((p, i) => `${a(p[0], stroke[i + 3][0])},${a(p[1], stroke[i + 3][1])}`)
        .join(' ');

      path.data(`${start}${middle} Z`);
    },
  };
};

export const rectangle: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const rect = new Konva.Rect({
    width: 0,
    height: 0,
    stroke: 'black',
    strokeWidth: 2,
    fill: 'white',
    seed: Math.random() * 2_147_483_637,
    sceneFunc: (context, shape) => {
      const { width: w, height: h, cornerRadius: r, stroke, strokeWidth, fill, seed } = shape.attrs;

      if (!w || !h) {
        return;
      }

      const d = `M ${r} 0 L ${w - r} 0 Q ${w} 0, ${w} ${r} L ${w} ${h - r} Q ${w} ${h}, ${w - r} ${h} L ${r} ${h} Q 0 ${h}, 0 ${h - r} L 0 ${r} Q 0 0, ${r} 0`;
      const drawable = generator.path(d, {
        roughness: 1,
        bowing: 1,
        stroke,
        strokeWidth,
        seed,
        fill,
        fillStyle: 'solid',
        fillWeight: strokeWidth / 2,
        hachureGap: strokeWidth * 4,
        preserveVertices: true,
      });

      context.lineJoin = 'round';
      context.lineCap = 'round';

      const paths = generator.toPaths(drawable);
      paths.forEach((pathInfo) => {
        const path = new Path2D(pathInfo.d);

        if (pathInfo.fill && pathInfo.fill !== 'none') {
          context.fillStyle = pathInfo.fill;
          context.fill(path);
        }

        if (pathInfo.stroke && pathInfo.stroke !== 'none') {
          context.strokeStyle = pathInfo.stroke;
          context.lineWidth = pathInfo.strokeWidth;
          context.stroke(path);
        }
      });
    },
    hitFunc(context, shape) {
      const { width: w, height: h, cornerRadius: r } = shape.attrs;

      if (!w || !h) {
        return;
      }

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
      context.closePath();
      context.fillStrokeShape(shape);
    },
  });

  canvas.scene.add(rect);

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const x = Math.min(anchor.x, head.x);
      const y = Math.min(anchor.y, head.y);
      const width = Math.abs(head.x - anchor.x);
      const height = Math.abs(head.y - anchor.y);

      const shortest = Math.min(width, height);
      const cornerRadius = shortest > 128 ? 32 : shortest * 0.25;

      rect.setAttrs({ x, y, width, height, cornerRadius });
    },
    destroy: () => {
      canvas.tf.nodes([rect]);
    },
  };
};

export const ellipse: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const ellipse = new Konva.Ellipse({
    radiusX: 0,
    radiusY: 0,
    stroke: 'black',
    strokeWidth: 2,
    fill: 'white',
    seed: Math.random() * 2_147_483_637,
    sceneFunc: (context, shape) => {
      const { radiusX, radiusY, stroke, strokeWidth, fill, seed } = shape.attrs;

      if (!radiusX || !radiusY) {
        return;
      }

      const drawable = generator.ellipse(0, 0, radiusX * 2, radiusY * 2, {
        roughness: 1,
        bowing: 1,
        stroke,
        strokeWidth,
        seed,
        fill,
        fillStyle: 'solid',
        fillWeight: strokeWidth / 2,
        hachureGap: strokeWidth * 4,
      });

      context.lineJoin = 'round';
      context.lineCap = 'round';

      const paths = generator.toPaths(drawable);
      paths.forEach((pathInfo) => {
        const path = new Path2D(pathInfo.d);

        if (pathInfo.fill && pathInfo.fill !== 'none') {
          context.fillStyle = pathInfo.fill;
          context.fill(path);
        }

        if (pathInfo.stroke && pathInfo.stroke !== 'none') {
          context.strokeStyle = pathInfo.stroke;
          context.lineWidth = pathInfo.strokeWidth;
          context.stroke(path);
        }
      });
    },
    hitFunc(context, shape) {
      const { radiusX, radiusY } = shape.attrs;

      if (!radiusX || !radiusY) {
        return;
      }

      context.beginPath();
      context.ellipse(0, 0, radiusX, radiusY, 0, 0, Math.PI * 2);
      context.closePath();
      context.fillStrokeShape(shape);
    },
  });

  canvas.scene.add(ellipse);

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const radiusX = Math.abs(head.x - anchor.x) / 2;
      const radiusY = Math.abs(head.y - anchor.y) / 2;

      const x = Math.min(anchor.x, head.x) + radiusX;
      const y = Math.min(anchor.y, head.y) + radiusY;

      ellipse.setAttrs({ x, y, radiusX, radiusY });
    },
    destroy: () => {
      canvas.tf.nodes([ellipse]);
    },
  };
};

export const line: Operation = (canvas) => {
  const anchor = canvas.stage.getRelativePointerPosition();
  if (!anchor) {
    return;
  }

  const line = new Konva.Line({
    points: [anchor.x, anchor.y],
    stroke: 'black',
    strokeWidth: 2,
    hitStrokeWidth: 10,
    seed: Math.random() * 2_147_483_637,
    sceneFunc: (context, shape) => {
      const { points, stroke, strokeWidth, seed } = shape.attrs;

      if (points.length < 4) {
        return;
      }

      const drawable = generator.line(points[0], points[1], points[2], points[3], {
        roughness: 1,
        bowing: 1,
        stroke,
        strokeWidth,
        seed,
      });

      context.lineJoin = 'round';
      context.lineCap = 'round';

      const paths = generator.toPaths(drawable);
      paths.forEach((pathInfo) => {
        const path = new Path2D(pathInfo.d);

        if (pathInfo.fill && pathInfo.fill !== 'none') {
          context.fillStyle = pathInfo.fill;
          context.fill(path);
        }

        if (pathInfo.stroke && pathInfo.stroke !== 'none') {
          context.strokeStyle = pathInfo.stroke;
          context.lineWidth = pathInfo.strokeWidth;
          context.stroke(path);
        }
      });
    },
    hitFunc(context, shape) {
      const { points, hitStrokeWidth } = shape.attrs;

      if (points.length < 4) {
        return;
      }

      const dx = points[2] - points[0];
      const dy = points[3] - points[1];
      const len = Math.hypot(dx, dy);
      const nx = ((-dy / len) * hitStrokeWidth) / 2;
      const ny = ((dx / len) * hitStrokeWidth) / 2;

      context.beginPath();
      context.moveTo(points[0] + nx, points[1] + ny);
      context.lineTo(points[2] + nx, points[3] + ny);
      context.lineTo(points[2] - nx, points[3] - ny);
      context.lineTo(points[0] - nx, points[1] - ny);
      context.closePath();
      context.fillStrokeShape(shape);
    },
  });

  canvas.scene.add(line);

  return {
    update: () => {
      const head = canvas.stage.getRelativePointerPosition();
      if (!head) {
        return;
      }

      const points = [anchor.x, anchor.y, head.x, head.y];

      line.setAttrs({ points });
    },
  };
};
