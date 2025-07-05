import Konva from 'konva';
import { clamp } from '$lib/utils';
import { Environment } from './environment';
import * as ops from './operations';
import { Scene } from './scene';
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

    this.#observer = new ResizeObserver(() => this.resize());
    this.#observer.observe(this.#container);

    this.#stage.on('pointerdown', (e) => this.#handlePointerDown(e.evt));
    this.#stage.on('pointermove', (e) => this.#handlePointerMove(e.evt));
    this.#stage.on('pointerup', (e) => this.#handlePointerUp(e.evt));
    this.#stage.on('pointercancel', (e) => this.#handlePointerUp(e.evt));
    this.#stage.on('wheel', (e) => this.#handleWheel(e.evt));
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

  get tf() {
    return this.#scene.transformer;
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

  #setOperation(operation: Operation, event: PointerEvent) {
    this.#operation?.destroy?.(event);

    const ret = operation(this, event);
    if (ret) {
      this.#operation = ret;
    }
  }

  #handlePointerDown(e: PointerEvent) {
    const pos = this.#stage.getPointerPosition();
    if (!pos) {
      return;
    }

    this.#pointers.set(e.pointerId, pos);

    const element = e.target as Element;
    element.setPointerCapture(e.pointerId);

    if (e.button === 1) {
      this.#setOperation(ops.pan, e);
      return;
    }

    const intersection = this.scene.getIntersection(pos);
    if (intersection) {
      if (this.tf.isAncestorOf(intersection)) {
        return;
      } else if (this.#state.tool === 'select') {
        this.#setOperation(ops.move, e);
        return;
      }
    }

    if (this.#state.tool === 'select') {
      this.#setOperation(ops.select, e);
    } else if (this.#state.tool === 'freedraw') {
      this.#setOperation(ops.freedraw, e);
    } else if (this.#state.tool === 'rectangle') {
      this.#setOperation(ops.rectangle, e);
    } else if (this.#state.tool === 'ellipse') {
      this.#setOperation(ops.ellipse, e);
    } else if (this.#state.tool === 'line') {
      this.#setOperation(ops.line, e);
    }
  }

  #handlePointerMove(e: PointerEvent) {
    const pos = this.#stage.getPointerPosition();
    if (!pos) {
      return;
    }

    this.#operation?.update?.(e);
  }

  #handlePointerUp(e: PointerEvent) {
    const element = e.target as Element;
    if (element.hasPointerCapture(e.pointerId)) {
      element.releasePointerCapture(e.pointerId);
    }

    this.#pointers.delete(e.pointerId);
    this.#operation?.destroy?.(e);
    this.#operation = null;
  }

  #handleWheel(e: WheelEvent) {
    e.preventDefault();

    if (e.ctrlKey || e.metaKey) {
      const pos = this.#stage.getPointerPosition();
      if (!pos) {
        return;
      }

      const multiplier = e.deltaMode === e.DOM_DELTA_PIXEL ? 0.01 : 0.05;
      const delta = Math.exp(-e.deltaY * multiplier);

      this.scaleBy(delta, { origin: pos });
    } else {
      this.moveBy(-e.deltaX, -e.deltaY);
    }
  }

  handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Backspace' || e.key === 'Delete') {
      const nodes = this.tf.nodes();
      this.tf.nodes([]);

      for (const node of nodes) {
        node.destroy();
      }
    } else if (e.key === 'Escape') {
      this.#state.tool = 'select';
      this.#operation?.destroy?.();
      this.#operation = null;
      this.#pointers.clear();
      this.tf.nodes([]);
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
      const nodes = this.scene.children.filter((child) => child !== this.tf);
      this.tf.nodes(nodes);
    }
  }
}
