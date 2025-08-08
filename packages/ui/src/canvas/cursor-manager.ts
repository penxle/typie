import Konva from 'konva';
import type { Awareness } from 'y-protocols/awareness';
import type { Canvas } from './class.svelte';

type UserInfo = {
  name: string;
  color: string;
};

type CursorState = {
  cursor: { x: number; y: number } | null;
  user?: UserInfo;
};

export class CursorManager {
  #awareness: Awareness;

  #stage: Konva.Stage;
  #layer: Konva.Layer;

  #cursors = new Map<number, Konva.Group>();

  constructor(canvas: Canvas, awareness: Awareness) {
    this.#awareness = awareness;

    this.#stage = canvas.stage;
    this.#layer = new Konva.Layer();
    canvas.stage.add(this.#layer);

    this.#awareness.setLocalStateField('cursor', null);
    this.#awareness.on('change', this.#handleAwarenessChange);
  }

  update() {
    const pos = this.#stage.getRelativePointerPosition();
    if (pos) {
      this.#awareness.setLocalStateField('cursor', { x: pos.x, y: pos.y });
    } else {
      this.#awareness.setLocalStateField('cursor', null);
    }

    const scale = this.#stage.scaleX();
    for (const cursor of this.#cursors.values()) {
      cursor.scale({
        x: 1 / scale,
        y: 1 / scale,
      });
    }
  }

  #handleAwarenessChange = () => {
    const states = this.#awareness.getStates();

    const activeCursors = new Set<number>();

    states.forEach((state, clientID) => {
      if (clientID === this.#awareness.clientID) return;

      const cursorState = state as CursorState;
      if (!cursorState.cursor || !cursorState.user) return;

      activeCursors.add(clientID);

      let cursorGroup = this.#cursors.get(clientID);
      if (!cursorGroup) {
        cursorGroup = this.#createCursor(cursorState.user);
        this.#cursors.set(clientID, cursorGroup);
        this.#layer.add(cursorGroup);
      }

      cursorGroup.position({
        x: cursorState.cursor.x,
        y: cursorState.cursor.y,
      });

      cursorGroup.visible(true);
    });

    this.#cursors.forEach((cursorGroup, clientID) => {
      if (!activeCursors.has(clientID)) {
        cursorGroup.visible(false);
      }
    });

    this.#layer.batchDraw();
  };

  #createCursor(user: UserInfo): Konva.Group {
    const group = new Konva.Group();

    const cursor = new Konva.Path({
      x: 0,
      y: 0,
      data: 'M5.5 3.21V20.8c0 .45.54.67.85.35l4.86-4.86a.5.5 0 0 1 .35-.15h6.87a.5.5 0 0 0 .35-.85L6.35 2.85a.5.5 0 0 0-.85.35Z',
      fill: user.color,
      stroke: 'white',
      strokeWidth: 1,
      shadowColor: 'rgba(0, 0, 0, 0.2)',
      shadowOffset: { x: 1, y: 1 },
      shadowBlur: 2,
    });

    const x = 20;
    const y = 20;
    const paddingX = 4;
    const paddingY = 4;

    const text = new Konva.Text({
      x: x + paddingX,
      y: y + paddingY,
      text: user.name,
      fontSize: 12,
      fontFamily: 'SUIT',
      fill: 'white',
    });

    const rect = new Konva.Rect({
      x,
      y,
      width: text.width() + paddingX * 2,
      height: text.height() + paddingY * 2,
      fill: user.color,
      cornerRadius: 4,
      shadowColor: 'rgba(0, 0, 0, 0.2)',
      shadowOffset: { x: 1, y: 1 },
      shadowBlur: 2,
    });

    group.add(cursor);
    group.add(rect);
    group.add(text);

    return group;
  }

  destroy() {
    this.#awareness.off('change', this.#handleAwarenessChange);
    this.#awareness.setLocalStateField('cursor', null);

    this.#layer.destroy();
  }
}
