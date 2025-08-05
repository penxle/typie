import type { Operation, Pos } from '../types';

export const pinch: Operation = (canvas, e) => {
  const pointers = new Map<number, Pos>();
  let lastDistance: number | null = null;

  if (e && e.evt.target instanceof Element) {
    const content = canvas.stage.getContent();
    if (content) {
      for (let i = 0; i < 10; i++) {
        if (content.hasPointerCapture(i)) {
          content.releasePointerCapture(i);
        }
      }
    }
  }

  return {
    update: (e) => {
      if (!e) return;

      const pos = canvas.stage.getPointerPosition();
      if (!pos) {
        return;
      }

      pointers.set(e.pointerId, pos);
      if (pointers.size === 2) {
        const pointerArray = [...pointers.values()];
        const [p1, p2] = pointerArray;

        const distance = Math.sqrt(Math.pow(p2.x - p1.x, 2) + Math.pow(p2.y - p1.y, 2));

        if (lastDistance !== null) {
          const delta = distance / lastDistance;
          const center = {
            x: (p1.x + p2.x) / 2,
            y: (p1.y + p2.y) / 2,
          };
          canvas.scaleBy(delta, { origin: center });
        }

        lastDistance = distance;
      }
    },
    destroy: () => {
      pointers.clear();
      lastDistance = null;
    },
  };
};
