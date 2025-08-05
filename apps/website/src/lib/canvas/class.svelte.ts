import Konva from 'konva';
import { nanoid } from 'nanoid';
import { SvelteMap } from 'svelte/reactivity';
import { match } from 'ts-pattern';
import * as Y from 'yjs';
import { clamp, Ref } from '$lib/utils';
import { copyShapesToClipboard, getShapesFromClipboard, offsetShapes } from './clipboard';
import { ARROW_MOVE_DISTANCE, ARROW_MOVE_DISTANCE_FAST, ARROW_PAN_DISTANCE, ARROW_PAN_DISTANCE_FAST } from './const';
import { CursorManager } from './cursor-manager';
import { Environment } from './environment';
import * as ops from './operations';
import { Scene } from './scene';
import { Selection } from './selection';
import { TypedArrow } from './shapes/arrow';
import { TypedBrush } from './shapes/brush';
import { TypedEllipse } from './shapes/ellipse';
import { TypedLine } from './shapes/line';
import { TypedRect } from './shapes/rectangle';
import { TypedStickyNote } from './shapes/stickynote';
import { SyncManager } from './sync-manager';
import type { Awareness } from 'y-protocols/awareness';
import type { Operation, OperationReturn, Pos, SerializedShape, Shapes, Tool } from './types';

type ScaleOptions = {
  origin: Pos;
};

export class CanvasState {
  #tool = $state<Tool>('select');
  #scale = $state(1);
  #selections = $state<Ref<Konva.Node>[]>([]);

  get tool() {
    return this.#tool;
  }

  set tool(value: Tool) {
    this.#tool = value;
  }

  get scale() {
    return this.#scale;
  }

  _setScale(value: number) {
    this.#scale = value;
  }

  get selections() {
    return this.#selections;
  }

  _setSelections(value: Konva.Node[]) {
    this.#selections = value.map((node) => new Ref(node));
  }
}

export class Canvas {
  #state = new CanvasState();

  #container: HTMLDivElement;

  #stage: Konva.Stage;
  #scene: Scene;
  #environment: Environment;
  #selection: Selection;

  #syncManager?: SyncManager;
  #cursorManager?: CursorManager;

  #observer: ResizeObserver;
  #pointers = new SvelteMap<number, Pos>();

  #operation: Partial<OperationReturn> | null = null;

  constructor(container: HTMLDivElement, doc?: Y.Doc, awareness?: Awareness) {
    this.#container = container;

    this.#stage = new Konva.Stage({
      container: this.#container,
      width: this.#container.offsetWidth,
      height: this.#container.offsetHeight,
    });

    this.#environment = new Environment(this.#stage);
    this.#scene = new Scene(this.#stage);
    this.#selection = new Selection(this.#stage, this.#state);

    if (doc) {
      this.#syncManager = new SyncManager(this, doc);

      if (awareness) {
        this.#cursorManager = new CursorManager(this, awareness);
      }
    }

    this.#observer = new ResizeObserver(() => this.resize());
    this.#observer.observe(this.#container);

    this.#stage.on('pointerdown', (e) => this.#handlePointerDown(e));
    this.#stage.on('pointermove', (e) => this.#handlePointerMove(e));
    this.#stage.on('pointerup', (e) => this.#handlePointerUp(e));
    this.#stage.on('pointercancel', (e) => this.#handlePointerUp(e));

    this.#stage.on('wheel', (e) => this.#handleWheel(e));

    this.#stage.on('attrchange', () => {
      this.selection.update();
      this.state._setSelections(this.#selection.nodes());
    });

    document.fonts.ready.then(() => {
      this.stage._requestDraw();
    });
  }

  get environment() {
    return this.#environment;
  }

  get state() {
    return this.#state;
  }

  get stage() {
    return this.#stage;
  }

  get scene() {
    return this.#scene.layer;
  }

  get selection() {
    return this.#selection;
  }

  get syncManager() {
    return this.#syncManager;
  }

  get cursorManager() {
    return this.#cursorManager;
  }

  resize() {
    this.#stage.width(this.#container.offsetWidth);
    this.#stage.height(this.#container.offsetHeight);
    this.#environment.update();
  }

  moveTo(x: number, y: number) {
    this.#stage.position({ x, y });
    this.#environment.update();
    this.#operation?.update?.();
    this.#cursorManager?.update();
  }

  moveBy(dx: number, dy: number) {
    const pos = this.#stage.position();
    this.moveTo(pos.x + dx, pos.y + dy);
  }

  scaleTo(target: number, options?: Partial<ScaleOptions>) {
    const origin = options?.origin ?? { x: this.#stage.width() / 2, y: this.#stage.height() / 2 };
    const value = clamp(target, 0.1, 5);

    const pos = this.#stage.position();
    const scale = this.#stage.scaleX();

    const x = origin.x - (origin.x - pos.x) * (value / scale);
    const y = origin.y - (origin.y - pos.y) * (value / scale);

    this.#stage.scale({ x: value, y: value });
    this.#stage.position({ x, y });

    this.#environment.update();
    this.#selection.update();

    this.#state._setScale(value);
    this.#operation?.update?.();

    this.#cursorManager?.update();
  }

  scaleBy(delta: number, options?: Partial<ScaleOptions>) {
    const scale = this.#stage.scaleX();
    this.scaleTo(scale * delta, options);
  }

  setCursor(cursor: string) {
    this.#container.style.cursor = cursor;
  }

  restoreCursor() {
    this.#container.style.cursor = '';
  }

  destroy() {
    this.#observer.disconnect();
    this.#cursorManager?.destroy();
    this.#stage.destroy();
  }

  setOperation(operation: Operation, event?: Konva.KonvaPointerEvent) {
    this.#operation?.destroy?.(event);

    const ret = operation(this, event);
    if (ret) {
      this.#operation = ret;
    }
  }

  #handlePointerDown(e: Konva.KonvaPointerEvent) {
    const pos = this.#stage.getPointerPosition();
    if (!pos) {
      return;
    }

    this.#pointers.set(e.pointerId, pos);

    if (this.#pointers.size === 2) {
      this.setOperation(ops.pinch, e);
      return;
    }

    if (e.evt.target instanceof Element) {
      e.evt.target.setPointerCapture(e.evt.pointerId);
    }

    if (e.evt.button === 1) {
      this.setOperation(ops.pan, e);
      return;
    }

    const handle = this.selection.handle(pos);
    if (handle) {
      this.setOperation(ops.createResizeOperation(handle), e);
      return;
    }

    if (this.#state.tool === 'select') {
      const isInsideSelection = this.selection.isInsideBoundingBox(pos);
      const intersectedShape = this.scene.getIntersection(pos);

      if (isInsideSelection || intersectedShape) {
        this.setOperation(ops.move, e);
      } else {
        this.setOperation(ops.select, e);
      }

      return;
    }

    if (this.#state.tool === 'pan') {
      this.setOperation(ops.pan, e);
      return;
    }

    this.selection.nodes([]);

    if (this.#state.tool === 'brush') {
      this.setOperation(ops.brush, e);
    } else if (this.#state.tool === 'rectangle') {
      this.setOperation(ops.rectangle, e);
    } else if (this.#state.tool === 'ellipse') {
      this.setOperation(ops.ellipse, e);
    } else if (this.#state.tool === 'line') {
      this.setOperation(ops.line, e);
    } else if (this.#state.tool === 'arrow') {
      this.setOperation(ops.arrow, e);
    } else if (this.#state.tool === 'stickynote') {
      this.setOperation(ops.stickynote, e);
    }
  }

  #handlePointerMove(e: Konva.KonvaPointerEvent) {
    const pos = this.#stage.getPointerPosition();
    if (!pos) {
      return;
    }

    this.#cursorManager?.update();
    this.#operation?.update?.(e);
  }

  #handlePointerUp(e: Konva.KonvaPointerEvent) {
    if (e.evt.target instanceof Element && e.evt.target.hasPointerCapture(e.evt.pointerId)) {
      e.evt.target.releasePointerCapture(e.evt.pointerId);
    }

    this.#pointers.delete(e.pointerId);
    this.#operation?.destroy?.(e);
    this.#operation = null;
  }

  #handleWheel(e: Konva.KonvaEventObject<WheelEvent>) {
    e.evt.preventDefault();

    if (e.evt.ctrlKey || e.evt.metaKey) {
      const pos = this.#stage.getPointerPosition();
      if (!pos) {
        return;
      }

      const multiplier = e.evt.deltaMode === e.evt.DOM_DELTA_PIXEL ? 0.01 : 0.02;
      const delta = Math.exp(-e.evt.deltaY * multiplier);

      this.scaleBy(delta, { origin: pos });
    } else {
      this.moveBy(-e.evt.deltaX, -e.evt.deltaY);
    }
  }

  async pasteShapesFromClipboard() {
    const shapes = await getShapesFromClipboard();
    if (!shapes || shapes.length === 0) return;

    const offsetShapesData = offsetShapes(shapes);
    const newNodes: Konva.Node[] = [];

    for (const shape of offsetShapesData) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const attrs = shape.attrs as any;
      const node = match(shape.type)
        .with('TypedRect', () => new TypedRect(attrs))
        .with('TypedEllipse', () => new TypedEllipse(attrs))
        .with('TypedLine', () => new TypedLine(attrs))
        .with('TypedArrow', () => new TypedArrow(attrs))
        .with('TypedBrush', () => new TypedBrush(attrs))
        .with('TypedStickyNote', () => new TypedStickyNote(attrs))
        .exhaustive();

      this.scene.add(node);
      this.#syncManager?.addOrUpdateKonvaNode(node);
      newNodes.push(node);
    }

    this.selection.nodes(newNodes);
    this.scene.batchDraw();
  }

  undo() {
    this.#syncManager?.undo();
    this.selection.nodes([]);
  }

  redo() {
    this.#syncManager?.redo();
    this.selection.nodes([]);
  }

  handleKeyDown(e: KeyboardEvent) {
    const activeElement = document.activeElement;
    if (
      activeElement &&
      (activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA' || activeElement.hasAttribute('contenteditable'))
    ) {
      return;
    }

    if (e.key === 'Backspace' || e.key === 'Delete') {
      e.preventDefault();

      const nodes = this.selection.nodes();
      this.selection.nodes([]);

      for (const node of nodes) {
        this.#syncManager?.removeKonvaNode(node);
        node.destroy();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      this.#state.tool = 'select';
      this.#operation?.destroy?.();
      this.#operation = null;
      this.#pointers.clear();
      this.selection.nodes([]);
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
      e.preventDefault();
      this.selection.nodes(this.scene.children);
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
      e.preventDefault();
      const nodes = this.selection.nodes();
      if (nodes.length > 0) {
        copyShapesToClipboard(nodes);
      }
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
      e.preventDefault();
      this.pasteShapesFromClipboard();
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
      e.preventDefault();
      this.undo();
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
      e.preventDefault();
      this.redo();
    } else if (e.key === 'Alt') {
      e.preventDefault();
      this.setCursor('copy');
    } else if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
      e.preventDefault();
      if (this.selection.nodes().length > 0) {
        this.moveSelectedShapesWithArrowKey(e);
      } else {
        this.panCanvasWithArrowKey(e);
      }
    }
  }

  handleKeyUp(e: KeyboardEvent) {
    if (e.key === 'Alt') {
      e.preventDefault();
      this.restoreCursor();
    }
  }

  moveSelectedShapesWithArrowKey(e: KeyboardEvent) {
    const moveDistance = e.shiftKey ? ARROW_MOVE_DISTANCE_FAST : ARROW_MOVE_DISTANCE;
    const { dx, dy } = this.getArrowKeyDelta(e.key, moveDistance);

    let selectedNodes = this.selection.nodes();
    const updatedShapes: { id: string; attrs: Record<string, unknown> }[] = [];

    const duplicate = e.altKey;

    if (duplicate) {
      const nodes = this.selection.nodes();
      const newNodes: Konva.Node[] = [];

      const shapes: SerializedShape[] = nodes.map((node) => ({
        type: node.className as Shapes,
        attrs: { ...node.attrs, id: nanoid(32) },
      }));

      for (const shape of shapes) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const attrs = shape.attrs as any;
        const node = match(shape.type)
          .with('TypedRect', () => new TypedRect(attrs))
          .with('TypedEllipse', () => new TypedEllipse(attrs))
          .with('TypedLine', () => new TypedLine(attrs))
          .with('TypedArrow', () => new TypedArrow(attrs))
          .with('TypedBrush', () => new TypedBrush(attrs))
          .with('TypedStickyNote', () => new TypedStickyNote(attrs))
          .exhaustive();

        this.scene.add(node);
        this.#syncManager?.addOrUpdateKonvaNode(node);
        newNodes.push(node);
      }
      selectedNodes = this.selection.nodes(newNodes);
    }

    for (const node of selectedNodes) {
      const newAttrs = {
        x: node.x() + dx,
        y: node.y() + dy,
      };

      updatedShapes.push({ id: node.id(), attrs: newAttrs });
    }

    if (updatedShapes.length > 0 && this.#syncManager) {
      for (const { id, attrs } of updatedShapes) {
        const node = selectedNodes.find((n) => n.id() === id);
        if (node) {
          node.setAttrs(attrs);
          this.#syncManager.addOrUpdateKonvaNode(node);
        }
      }
      this.selection.update();
    }
  }

  panCanvasWithArrowKey(e: KeyboardEvent) {
    const panDistance = e.shiftKey ? ARROW_PAN_DISTANCE_FAST : ARROW_PAN_DISTANCE;
    const { dx, dy } = this.getArrowKeyDelta(e.key, panDistance);

    this.moveBy(-dx, -dy);
  }

  private getArrowKeyDelta(key: string, distance: number): { dx: number; dy: number } {
    switch (key) {
      case 'ArrowUp': {
        return { dx: 0, dy: -distance };
      }
      case 'ArrowDown': {
        return { dx: 0, dy: distance };
      }
      case 'ArrowLeft': {
        return { dx: -distance, dy: 0 };
      }
      case 'ArrowRight': {
        return { dx: distance, dy: 0 };
      }
      default: {
        return { dx: 0, dy: 0 };
      }
    }
  }
}
