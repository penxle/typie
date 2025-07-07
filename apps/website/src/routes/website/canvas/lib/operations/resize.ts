import { MIN_SIZE } from '../const';
import { TypedEllipse, TypedRect, TypedStickyNote } from '../shapes';
import { TypedBrush } from '../shapes/brush';
import { TypedLine } from '../shapes/line';
import type { Operation, ResizeHandle } from '../types';

type NodeState =
  | { type: 'rect'; node: TypedRect; x: number; y: number; width: number; height: number }
  | { type: 'ellipse'; node: TypedEllipse; x: number; y: number; radiusX: number; radiusY: number }
  | { type: 'line'; node: TypedLine; x: number; y: number; dx: number; dy: number }
  | { type: 'brush'; node: TypedBrush; x: number; y: number; points: [number, number][] }
  | { type: 'stickynote'; node: TypedStickyNote; x: number; y: number; width: number; height: number };

const HANDLES: Record<ResizeHandle, [number, number, number, number]> = {
  t: [0, -1, 0.5, 0],
  r: [1, 0, 1, 0.5],
  b: [0, 1, 0.5, 1],
  l: [-1, 0, 0, 0.5],
  tl: [-1, -1, 0, 0],
  tr: [1, -1, 1, 0],
  br: [1, 1, 1, 1],
  bl: [-1, 1, 0, 1],
};

export const createResizeOperation =
  (handle: ResizeHandle): Operation =>
  (canvas, event) => {
    const nodes = canvas.selection.nodes();
    if (nodes.length === 0) return;

    event?.target.setPointerCapture(event.pointerId);

    const states: NodeState[] = [];
    const min = { x: Infinity, y: Infinity };
    const max = { x: -Infinity, y: -Infinity };

    for (const node of nodes) {
      if (node instanceof TypedRect) {
        const { x, y, width, height } = node.attrs;
        states.push({ type: 'rect', node, x, y, width, height });
        min.x = Math.min(min.x, x);
        min.y = Math.min(min.y, y);
        max.x = Math.max(max.x, x + width);
        max.y = Math.max(max.y, y + height);
      } else if (node instanceof TypedEllipse) {
        const { x, y, radiusX, radiusY } = node.attrs;
        states.push({ type: 'ellipse', node, x, y, radiusX, radiusY });
        min.x = Math.min(min.x, x - radiusX);
        min.y = Math.min(min.y, y - radiusY);
        max.x = Math.max(max.x, x + radiusX);
        max.y = Math.max(max.y, y + radiusY);
      } else if (node instanceof TypedLine) {
        const { x, y, dx, dy } = node.attrs;
        states.push({ type: 'line', node, x, y, dx, dy });
        min.x = Math.min(min.x, x, x + dx);
        min.y = Math.min(min.y, y, y + dy);
        max.x = Math.max(max.x, x, x + dx);
        max.y = Math.max(max.y, y, y + dy);
      } else if (node instanceof TypedBrush) {
        const { x, y, points } = node.attrs;
        states.push({ type: 'brush', node, x, y, points: [...points] });
        const selfRect = node.getSelfRect();
        min.x = Math.min(min.x, x + selfRect.x);
        min.y = Math.min(min.y, y + selfRect.y);
        max.x = Math.max(max.x, x + selfRect.x + selfRect.width);
        max.y = Math.max(max.y, y + selfRect.y + selfRect.height);
      } else if (node instanceof TypedStickyNote) {
        const { x, y, width, height } = node.attrs;
        states.push({ type: 'stickynote', node, x, y, width, height });
        min.x = Math.min(min.x, x);
        min.y = Math.min(min.y, y);
        max.x = Math.max(max.x, x + width);
        max.y = Math.max(max.y, y + height);
      }
    }

    if (!Number.isFinite(min.x) || !Number.isFinite(min.y)) return;

    const initBbox = { min, max };
    const initSize = { w: max.x - min.x, h: max.y - min.y };
    const initCenter = { x: (min.x + max.x) / 2, y: (min.y + max.y) / 2 };
    const ratio = initSize.w / initSize.h;

    const hdl = HANDLES[handle];
    const dragStart = {
      x: initBbox.min.x + hdl[2] * initSize.w,
      y: initBbox.min.y + hdl[3] * initSize.h,
    };

    const minSize = { w: 0, h: 0 };
    for (const state of states) {
      if (state.type === 'rect') {
        minSize.w = Math.max(minSize.w, (MIN_SIZE * initSize.w) / state.width);
        minSize.h = Math.max(minSize.h, (MIN_SIZE * initSize.h) / state.height);
      } else if (state.type === 'ellipse') {
        minSize.w = Math.max(minSize.w, (MIN_SIZE * initSize.w) / state.radiusX / 2);
        minSize.h = Math.max(minSize.h, (MIN_SIZE * initSize.h) / state.radiusY / 2);
      }
    }

    return {
      update: (event) => {
        const mouse = canvas.stage.getRelativePointerPosition();
        if (!mouse) return;

        const shift = event?.evt.shiftKey ?? false;
        const alt = event?.evt.altKey ?? false;

        const delta = { x: mouse.x - dragStart.x, y: mouse.y - dragStart.y };
        const dir = { x: hdl[0], y: hdl[1] };

        let bbox = {
          min: { ...initBbox.min },
          max: { ...initBbox.max },
        };

        if (alt) {
          if (dir.x !== 0) {
            const dx = dir.x < 0 ? delta.x : -delta.x;
            bbox.min.x = initBbox.min.x + dx;
            bbox.max.x = initBbox.max.x - dx;
          }
          if (dir.y !== 0) {
            const dy = dir.y < 0 ? delta.y : -delta.y;
            bbox.min.y = initBbox.min.y + dy;
            bbox.max.y = initBbox.max.y - dy;
          }
          const size = { w: bbox.max.x - bbox.min.x, h: bbox.max.y - bbox.min.y };
          if (size.w < minSize.w) {
            bbox.min.x = initCenter.x - minSize.w / 2;
            bbox.max.x = initCenter.x + minSize.w / 2;
          }
          if (size.h < minSize.h) {
            bbox.min.y = initCenter.y - minSize.h / 2;
            bbox.max.y = initCenter.y + minSize.h / 2;
          }
        } else {
          if (dir.x < 0) bbox.min.x = initBbox.min.x + delta.x;
          if (dir.x > 0) bbox.max.x = initBbox.max.x + delta.x;
          if (dir.y < 0) bbox.min.y = initBbox.min.y + delta.y;
          if (dir.y > 0) bbox.max.y = initBbox.max.y + delta.y;
        }

        if (shift) {
          const mouseBbox = {
            min: { ...initBbox.min },
            max: { ...initBbox.max },
          };

          if (alt) {
            if (dir.x !== 0) {
              const dx = dir.x < 0 ? delta.x : -delta.x;
              mouseBbox.min.x = initBbox.min.x + dx;
              mouseBbox.max.x = initBbox.max.x - dx;
            }
            if (dir.y !== 0) {
              const dy = dir.y < 0 ? delta.y : -delta.y;
              mouseBbox.min.y = initBbox.min.y + dy;
              mouseBbox.max.y = initBbox.max.y - dy;
            }
          } else {
            if (dir.x < 0) mouseBbox.min.x = initBbox.min.x + delta.x;
            if (dir.x > 0) mouseBbox.max.x = initBbox.max.x + delta.x;
            if (dir.y < 0) mouseBbox.min.y = initBbox.min.y + delta.y;
            if (dir.y > 0) mouseBbox.max.y = initBbox.max.y + delta.y;
          }

          const mouseSize = {
            w: mouseBbox.max.x - mouseBbox.min.x,
            h: mouseBbox.max.y - mouseBbox.min.y,
          };
          const finalSize = { ...mouseSize };

          if (dir.x !== 0 && dir.y !== 0) {
            if (mouseSize.w < 0 || mouseSize.h < 0) {
              const minScale = Math.max(minSize.w / initSize.w, minSize.h / initSize.h);
              finalSize.w = initSize.w * minScale;
              finalSize.h = initSize.h * minScale;
            } else {
              const scale = Math.min(mouseSize.w / initSize.w, mouseSize.h / initSize.h);
              const minScale = Math.max(minSize.w / initSize.w, minSize.h / initSize.h);
              const finalScale = Math.max(scale, minScale);
              finalSize.w = initSize.w * finalScale;
              finalSize.h = initSize.h * finalScale;
            }
          } else if (dir.x === 0) {
            finalSize.h = Math.max(Math.abs(mouseSize.h), minSize.h);
            finalSize.w = Math.max(finalSize.h * ratio, minSize.w);
          } else {
            finalSize.w = Math.max(Math.abs(mouseSize.w), minSize.w);
            finalSize.h = Math.max(finalSize.w / ratio, minSize.h);
          }

          if (alt) {
            bbox = {
              min: { x: initCenter.x - finalSize.w / 2, y: initCenter.y - finalSize.h / 2 },
              max: { x: initCenter.x + finalSize.w / 2, y: initCenter.y + finalSize.h / 2 },
            };
          } else {
            const anchor = {
              x: initBbox.min.x + (1 - hdl[2]) * initSize.w,
              y: initBbox.min.y + (1 - hdl[3]) * initSize.h,
            };

            if (dir.x === 0) {
              const center = (initBbox.min.x + initBbox.max.x) / 2;
              bbox.min.x = center - finalSize.w / 2;
              bbox.max.x = center + finalSize.w / 2;
            } else {
              bbox.min.x = dir.x < 0 ? anchor.x - finalSize.w : anchor.x;
              bbox.max.x = dir.x < 0 ? anchor.x : anchor.x + finalSize.w;
            }

            if (dir.y === 0) {
              const center = (initBbox.min.y + initBbox.max.y) / 2;
              bbox.min.y = center - finalSize.h / 2;
              bbox.max.y = center + finalSize.h / 2;
            } else {
              bbox.min.y = dir.y < 0 ? anchor.y - finalSize.h : anchor.y;
              bbox.max.y = dir.y < 0 ? anchor.y : anchor.y + finalSize.h;
            }
          }
        }

        const size = { w: bbox.max.x - bbox.min.x, h: bbox.max.y - bbox.min.y };
        if (size.w < minSize.w) {
          if (dir.x < 0) bbox.min.x = bbox.max.x - minSize.w;
          else if (dir.x > 0) bbox.max.x = bbox.min.x + minSize.w;
        }
        if (size.h < minSize.h) {
          if (dir.y < 0) bbox.min.y = bbox.max.y - minSize.h;
          else if (dir.y > 0) bbox.max.y = bbox.min.y + minSize.h;
        }

        const scale = {
          x: (bbox.max.x - bbox.min.x) / initSize.w,
          y: (bbox.max.y - bbox.min.y) / initSize.h,
        };

        for (const state of states) {
          const rel = {
            x: (state.x - initBbox.min.x) / initSize.w,
            y: (state.y - initBbox.min.y) / initSize.h,
          };
          const newPos = {
            x: bbox.min.x + rel.x * (bbox.max.x - bbox.min.x),
            y: bbox.min.y + rel.y * (bbox.max.y - bbox.min.y),
          };

          switch (state.type) {
            case 'rect': {
              state.node.setAttrs({
                x: newPos.x,
                y: newPos.y,
                width: Math.max(state.width * scale.x, MIN_SIZE),
                height: Math.max(state.height * scale.y, MIN_SIZE),
              });

              break;
            }
            case 'ellipse': {
              state.node.setAttrs({
                x: newPos.x,
                y: newPos.y,
                radiusX: Math.max(state.radiusX * scale.x, MIN_SIZE / 2),
                radiusY: Math.max(state.radiusY * scale.y, MIN_SIZE / 2),
              });

              break;
            }
            case 'line': {
              state.node.setAttrs({
                x: newPos.x,
                y: newPos.y,
                dx: state.dx * scale.x,
                dy: state.dy * scale.y,
              });

              break;
            }
            case 'brush': {
              state.node.setAttrs({
                x: newPos.x,
                y: newPos.y,
                points: state.points.map(([px, py]) => [px * scale.x, py * scale.y] as [number, number]),
              });

              break;
            }
            case 'stickynote': {
              state.node.setAttrs({
                x: newPos.x,
                y: newPos.y,
                width: Math.max(state.width * scale.x, MIN_SIZE),
                height: Math.max(state.height * scale.y, MIN_SIZE),
              });

              break;
            }
          }
        }

        canvas.selection.update();
      },
      destroy: () => {
        event?.target.releaseCapture(event.pointerId);
        canvas.state.tool = 'select';
      },
    };
  };
