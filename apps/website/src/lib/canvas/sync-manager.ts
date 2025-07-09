import Konva from 'konva';
import { match } from 'ts-pattern';
import * as Y from 'yjs';
import { TypedArrow } from './shapes/arrow';
import { TypedBrush } from './shapes/brush';
import { TypedEllipse } from './shapes/ellipse';
import { TypedLine } from './shapes/line';
import { TypedRect } from './shapes/rectangle';
import { TypedStickyNote } from './shapes/stickynote';
import type { Canvas } from './class.svelte';
import type { Shapes } from './types';

export type YShape = {
  type: Shapes;
  attrs: Record<string, unknown>;
};

export class SyncManager {
  #canvas: Canvas;

  #doc: Y.Doc;
  #shapes: Y.Map<YShape>;
  #orders: Y.Array<string>;

  #isUpdating = false;
  #nodeIdMap = new Map<string, Konva.Node>();

  constructor(canvas: Canvas, doc: Y.Doc) {
    this.#canvas = canvas;

    this.#doc = doc;
    this.#shapes = this.#doc.getMap('shapes');
    this.#orders = this.#doc.getArray('orders');

    this.#shapes.observe((event) => {
      if (this.#isUpdating) return;

      event.keysChanged.forEach((id) => {
        const change = event.changes.keys.get(id);
        if (!change) return;

        if (change.action === 'add' || change.action === 'update') {
          const shape = this.#shapes.get(id);
          if (shape) {
            const node = this.#nodeIdMap.get(id);
            if (node) {
              this.#isUpdating = true;
              node.setAttrs(shape.attrs);
              this.#isUpdating = false;
            } else {
              this.#createKonvaNode(id, shape);
            }

            this.#canvas.scene.batchDraw();
          }
        } else if (change.action === 'delete') {
          const node = this.#nodeIdMap.get(id);
          if (node) {
            node.destroy();
            this.#nodeIdMap.delete(id);
            this.#canvas.scene.batchDraw();
          }
        }
      });
    });

    this.#orders.observe(() => {
      if (this.#isUpdating) return;

      const order = this.#orders.toArray();
      order.forEach((id, index) => {
        const node = this.#nodeIdMap.get(id);
        if (node) {
          node.setZIndex(index);
        }
      });

      this.#canvas.scene.batchDraw();
    });

    for (const id of this.#orders.toArray()) {
      const shape = this.#shapes.get(id);
      if (shape) {
        this.#createKonvaNode(id, shape);
      }
    }
  }

  #updateShape(id: string, attrs: Record<string, unknown>) {
    const shape = this.#shapes.get(id);
    if (!shape) return;

    this.#doc.transact(() => {
      this.#shapes.set(id, { ...shape, attrs });
    });
  }

  #createKonvaNode(id: string, shape: YShape) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const attrs = { ...shape.attrs, id } as any;
    const node = match(shape.type)
      .with('rectangle', () => new TypedRect(attrs))
      .with('ellipse', () => new TypedEllipse(attrs))
      .with('line', () => new TypedLine(attrs))
      .with('arrow', () => new TypedArrow(attrs))
      .with('brush', () => new TypedBrush(attrs))
      .with('stickynote', () => new TypedStickyNote(attrs))
      .exhaustive();

    this.#nodeIdMap.set(id, node);
    this.#canvas.scene.add(node);
    this.#setupNodeListeners(id);
  }

  #setupNodeListeners(id: string) {
    const node = this.#nodeIdMap.get(id);
    if (!node) return;

    node.on('attrchange', () => {
      if (this.#isUpdating) return;

      this.#isUpdating = true;
      this.#updateShape(id, node.attrs);
      this.#isUpdating = false;
    });
  }

  addOrUpdateKonvaNode(node: Konva.Node) {
    const { id, type, ...attrs } = node.attrs;

    this.#isUpdating = true;

    if (this.#nodeIdMap.has(id)) {
      this.#updateShape(id, attrs);
    } else {
      this.#doc.transact(() => {
        this.#shapes.set(id, { type, attrs });
        this.#orders.push([id]);
      });

      this.#nodeIdMap.set(id, node);
      this.#setupNodeListeners(id);
    }

    this.#isUpdating = false;
  }

  removeKonvaNode(node: Konva.Node) {
    const { id } = node.attrs;

    this.#isUpdating = true;

    this.#doc.transact(() => {
      this.#shapes.delete(id);

      const index = this.#orders.toArray().indexOf(id);
      if (index !== -1) {
        this.#orders.delete(index, 1);
      }
    });

    this.#nodeIdMap.delete(id);

    this.#isUpdating = false;
  }
}
