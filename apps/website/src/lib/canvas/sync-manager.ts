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

type Attrs = {
  id: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
};

type Shape = {
  type: Shapes;
  attrs: Attrs;
};

export class SyncManager {
  #canvas: Canvas;

  #doc: Y.Doc;
  #fragment: Y.XmlFragment;
  #undoManager: Y.UndoManager;

  #nodeMap = new Map<string, Konva.Node>();
  #elementMap = new Map<string, Y.XmlElement>();

  #isUpdating = false;

  constructor(canvas: Canvas, doc: Y.Doc) {
    this.#canvas = canvas;

    this.#doc = doc;
    this.#fragment = this.#doc.getXmlFragment('shapes');

    this.#undoManager = new Y.UndoManager([this.#fragment], {
      captureTimeout: 500,
      trackedOrigins: new Set(['local']),
    });

    this.#fragment.observeDeep((events) => {
      events.forEach((event) => {
        if (event.transaction.origin === 'local') return;

        if (event.target instanceof Y.XmlElement) {
          this.#handleXmlElementChange(event);
        } else if (event.target instanceof Y.XmlFragment) {
          this.#handleXmlFragmentChange(event);
        }
      });
    });

    this.#syncNewElements();
    this.#updateZIndices();
  }

  #getElement(id: string) {
    const element = this.#elementMap.get(id);
    if (element) {
      return element;
    }

    this.#fragment.forEach((element) => {
      if (element instanceof Y.XmlElement && this.#getElementId(element) === id) {
        this.#elementMap.set(id, element);
        return element;
      }
    });

    return null;
  }

  #getElementId(element: Y.XmlElement) {
    const id = element.getAttribute('id');
    if (!id) throw new Error('Element has no id');
    return JSON.parse(id) as string;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  #updateShape(id: string, attrs: Record<string, any>) {
    const element = this.#getElement(id);
    if (!element) return;

    this.#doc.transact(() => {
      Object.entries(attrs).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          const v = JSON.stringify(value);
          if (element.getAttribute(key) !== v) {
            element.setAttribute(key, v);
          }
        } else {
          element.removeAttribute(key);
        }
      });
    }, 'local');
  }

  #elementToShape(element: Y.XmlElement) {
    const type = element.nodeName as Shapes;
    if (!type) return null;

    const attrs: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(element.getAttributes())) {
      if (value) {
        attrs[key] = JSON.parse(value);
      }
    }

    return { type, attrs } as Shape;
  }

  #handleXmlElementChange(event: Y.YEvent<Y.XmlElement>) {
    const id = this.#getElementId(event.target);

    if (event.changes.added.size > 0) {
      this.#addElementToCanvas(id, event.target);
    }

    if (event.changes.deleted.size > 0) {
      this.#removeElementFromCanvas(id);
    }

    if (event.changes.keys.size > 0) {
      this.#updateElementAttributes(id, event.target);
    }
  }

  #handleXmlFragmentChange(event: Y.YEvent<Y.XmlFragment>) {
    if (event.changes.added.size > 0) {
      this.#syncNewElements();
    }

    if (event.changes.added.size > 0 || event.changes.deleted.size > 0) {
      this.#updateZIndices();
    }

    if (event.changes.deleted.size > 0) {
      const existingIds = new Set<string>();

      this.#fragment.forEach((element) => {
        if (element instanceof Y.XmlElement) {
          const id = this.#getElementId(element);
          existingIds.add(id);
        }
      });

      for (const id of this.#elementMap.keys()) {
        if (!existingIds.has(id)) {
          this.#removeElementFromCanvas(id);
        }
      }
    }
  }

  #addElementToCanvas(id: string, xmlElement: Y.XmlElement) {
    if (this.#nodeMap.has(id)) return;

    const shape = this.#elementToShape(xmlElement);
    if (shape) {
      this.#createKonvaNode(id, shape);
      this.#elementMap.set(id, xmlElement);
    }
  }

  #removeElementFromCanvas(id: string) {
    const node = this.#nodeMap.get(id);
    if (node) {
      node.destroy();
      this.#nodeMap.delete(id);
      this.#elementMap.delete(id);
    }
  }

  #updateElementAttributes(id: string, element: Y.XmlElement) {
    const node = this.#nodeMap.get(id);
    const shape = this.#elementToShape(element);

    if (node && shape) {
      this.#isUpdating = true;
      node.setAttrs(shape.attrs);
      this.#isUpdating = false;
    }
  }

  #syncNewElements() {
    this.#fragment.forEach((element) => {
      if (element instanceof Y.XmlElement) {
        const id = this.#getElementId(element);
        this.#addElementToCanvas(id, element);
      }
    });
  }

  #updateZIndices() {
    const elements = this.#fragment.toArray();
    elements.forEach((element, index) => {
      if (element instanceof Y.XmlElement) {
        const id = this.#getElementId(element);
        const node = this.#nodeMap.get(id);
        if (node) {
          node.setZIndex(index);
        }
      }
    });
  }

  #createKonvaNode(id: string, shape: Shape) {
    this.#isUpdating = true;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const attrs = { ...shape.attrs, id } as any;
    const node = match(shape.type)
      .with('TypedRect', () => new TypedRect(attrs))
      .with('TypedEllipse', () => new TypedEllipse(attrs))
      .with('TypedLine', () => new TypedLine(attrs))
      .with('TypedArrow', () => new TypedArrow(attrs))
      .with('TypedBrush', () => new TypedBrush(attrs))
      .with('TypedStickyNote', () => new TypedStickyNote(attrs))
      .exhaustive();

    this.#nodeMap.set(id, node);
    this.#canvas.scene.add(node);
    this.#setupNodeListeners(id);

    this.#isUpdating = false;
  }

  #setupNodeListeners(id: string) {
    const node = this.#nodeMap.get(id);
    if (!node) return;

    node.on('attrchange', () => {
      if (this.#isUpdating) return;

      this.#isUpdating = true;
      this.#updateShape(id, node.attrs);
      this.#isUpdating = false;
    });
  }

  addOrUpdateKonvaNode(node: Konva.Node) {
    const { id, ...attrs } = node.attrs;

    if (this.#elementMap.has(id)) {
      this.#updateShape(id, attrs);
    } else {
      this.#doc.transact(() => {
        const element = new Y.XmlElement(node.className);
        element.setAttribute('id', JSON.stringify(id));

        Object.entries(attrs).forEach(([key, value]) => {
          if (value !== undefined && value !== null) {
            element.setAttribute(key, JSON.stringify(value));
          }
        });

        this.#fragment.push([element]);
        this.#elementMap.set(id, element);
      }, 'local');

      this.#nodeMap.set(id, node);
      this.#setupNodeListeners(id);
    }
  }

  removeKonvaNode(node: Konva.Node) {
    const { id } = node.attrs;

    const element = this.#elementMap.get(id);
    if (element) {
      this.#doc.transact(() => {
        const children = this.#fragment.toArray();
        const index = children.indexOf(element);
        if (index !== -1) {
          this.#fragment.delete(index, 1);
        }
      }, 'local');

      this.#elementMap.delete(id);
    }

    this.#nodeMap.delete(id);
  }

  undo() {
    this.#undoManager.undo();
  }

  redo() {
    this.#undoManager.redo();
  }
}
