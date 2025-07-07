import Konva from 'konva';
import { clamp } from '$lib/utils';
import { Environment } from './environment';
import * as ops from './operations';
import { Scene } from './scene';
import { Selection } from './selection';
import type { Operation, OperationReturn, Pos, Tool } from './types';

type ScaleOptions = {
  origin: Pos;
};

export class CanvasState {
  #tool = $state<Tool>('select');
  #scale = $state(1);

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
}

export class Canvas {
  #state = new CanvasState();

  #container: HTMLDivElement;

  #stage: Konva.Stage;
  #scene: Scene;
  #environment: Environment;
  #selection: Selection;

  #observer: ResizeObserver;
  #pointers = new Map<number, Pos>();

  #operation: Partial<OperationReturn> | null = null;

  constructor(container: HTMLDivElement) {
    this.#container = container;

    this.#stage = new Konva.Stage({
      container: this.#container,
      width: this.#container.offsetWidth,
      height: this.#container.offsetHeight,
    });

    this.#environment = new Environment(this.#stage);
    this.#scene = new Scene(this.#stage);
    this.#selection = new Selection(this.#stage);

    this.#observer = new ResizeObserver(() => this.resize());
    this.#observer.observe(this.#container);

    this.#stage.on('pointerdown', (e) => this.#handlePointerDown(e));
    this.#stage.on('pointermove', (e) => this.#handlePointerMove(e));
    this.#stage.on('pointerup', (e) => this.#handlePointerUp(e));
    this.#stage.on('pointercancel', (e) => this.#handlePointerUp(e));
    this.#stage.on('wheel', (e) => this.#handleWheel(e));
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

  resize() {
    this.#stage.width(this.#container.offsetWidth);
    this.#stage.height(this.#container.offsetHeight);
    this.#environment.update();
  }

  moveTo(x: number, y: number) {
    this.#stage.position({ x, y });
    this.#environment.update();
    this.#operation?.update?.();
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
  }

  scaleBy(delta: number, options?: Partial<ScaleOptions>) {
    const scale = this.#stage.scaleX();
    this.scaleTo(scale * delta, options);
  }

  destroy() {
    this.#observer.disconnect();
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

    e.target?.setPointerCapture(e.pointerId);
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
    } else if (this.#state.tool === 'brush') {
      this.setOperation(ops.brush, e);
    } else if (this.#state.tool === 'rectangle') {
      this.setOperation(ops.rectangle, e);
    } else if (this.#state.tool === 'ellipse') {
      this.setOperation(ops.ellipse, e);
    } else if (this.#state.tool === 'line') {
      this.setOperation(ops.line, e);
    } else if (this.#state.tool === 'stickynote') {
      this.setOperation(ops.stickynote, e);
    }
  }

  #handlePointerMove(e: Konva.KonvaPointerEvent) {
    const pos = this.#stage.getPointerPosition();
    if (!pos) {
      return;
    }

    this.#operation?.update?.(e);
  }

  #handlePointerUp(e: Konva.KonvaPointerEvent) {
    if (e.target?.hasPointerCapture(e.pointerId)) {
      e.target.releaseCapture(e.pointerId);
    }

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

  handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Backspace' || e.key === 'Delete') {
      e.preventDefault();

      const nodes = this.selection.nodes();
      this.selection.nodes([]);

      for (const node of nodes) {
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
    }
  }
}
