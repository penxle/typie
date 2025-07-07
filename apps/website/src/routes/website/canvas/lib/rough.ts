import rough from 'roughjs';
import type Konva from 'konva';
import type { Drawable } from 'roughjs/bin/core';

export const roughGenerator = rough.generator();

export const renderRoughDrawable = (context: Konva.Context, drawable: Drawable) => {
  context.lineJoin = 'round';
  context.lineCap = 'round';

  const paths = roughGenerator.toPaths(drawable);
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
};
