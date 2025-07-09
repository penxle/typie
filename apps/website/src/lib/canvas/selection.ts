import Konva from 'konva';
import { isSingleLineLike } from './utils';
import type { CanvasState } from './class.svelte';
import type { Pos, ResizeHandle, ResizeLineHandle, ResizeRectHandle } from './types';

const CORNER_RECT_SIZE = 12;
const CORNER_RECT_STROKE_WIDTH = 2;
const EDGE_LINE_STROKE_WIDTH = 2;

export class Selection {
  #stage: Konva.Stage;
  #layer: Konva.Layer;

  #container: HTMLDivElement;
  #state: CanvasState;

  #indicator: Konva.Rect;

  #rectHandlesGroup: Konva.Group;
  #lineHandlesGroup: Konva.Group;

  #rectHandles = new Map<ResizeRectHandle, Konva.Shape>();
  #lineHandles = new Map<ResizeLineHandle, Konva.Circle>();

  #nodes: Konva.Node[] = [];

  constructor(stage: Konva.Stage, state: CanvasState) {
    this.#stage = stage;
    this.#state = state;
    this.#container = this.#stage.container();
    this.#layer = new Konva.Layer();
    this.#stage.add(this.#layer);

    this.#rectHandlesGroup = new Konva.Group();
    this.#lineHandlesGroup = new Konva.Group();
    this.#layer.add(this.#rectHandlesGroup);
    this.#layer.add(this.#lineHandlesGroup);

    this.#indicator = new Konva.Rect({
      stroke: 'rgba(0, 135, 255, 1)',
      strokeWidth: 0.5,
      fill: 'rgba(0, 135, 255, 0.1)',
      strokeScaleEnabled: false,
    });

    this.#layer.add(this.#indicator);

    const edges: ResizeRectHandle[] = ['t', 'r', 'b', 'l'];
    for (const edge of edges) {
      const line = new Konva.Line({
        stroke: '#0087ff',
        handle: edge,
      });

      line.on('pointerenter', () => {
        this.#container.style.cursor = edge === 't' || edge === 'b' ? 'ns-resize' : 'ew-resize';
      });

      line.on('pointerleave', () => {
        this.#container.style.cursor = '';
      });

      this.#rectHandles.set(edge, line);
      this.#rectHandlesGroup.add(line);
    }

    const corners: ResizeRectHandle[] = ['tl', 'tr', 'br', 'bl'];
    for (const corner of corners) {
      const rect = new Konva.Rect({
        fill: 'white',
        stroke: '#0087ff',
        handle: corner,
      });

      rect.on('pointerenter', () => {
        // spell-checker:disable-next-line
        this.#container.style.cursor = corner === 'tl' || corner === 'br' ? 'nwse-resize' : 'nesw-resize';
      });

      rect.on('pointerleave', () => {
        this.#container.style.cursor = '';
      });

      this.#rectHandles.set(corner, rect);
      this.#rectHandlesGroup.add(rect);
    }

    const endpoints: ResizeLineHandle[] = ['start', 'end'];
    for (const endpoint of endpoints) {
      const circle = new Konva.Circle({
        fill: 'white',
        stroke: '#0087ff',
        handle: endpoint,
      });

      circle.on('pointerenter', () => {
        this.#container.style.cursor = 'move';
      });

      circle.on('pointerleave', () => {
        this.#container.style.cursor = '';
      });

      this.#lineHandles.set(endpoint, circle);
      this.#lineHandlesGroup.add(circle);
    }
  }

  getIndicatorClientRect() {
    return this.#indicator.getClientRect();
  }

  showIndicator(x: number, y: number, width: number, height: number) {
    this.#indicator.setAttrs({
      x,
      y,
      width,
      height,
      visible: true,
    });
  }

  hideIndicator() {
    this.#indicator.visible(false);
  }

  nodes(newNodes?: Konva.Node[]) {
    if (newNodes !== undefined) {
      this.#state._setSelections(newNodes);
      this.#nodes = [...newNodes];
      this.update();
    }

    return this.#nodes;
  }

  contains(node: Konva.Node) {
    return this.#nodes.includes(node);
  }

  update() {
    if (this.#nodes.length === 0) {
      this.#rectHandlesGroup.visible(false);
      this.#lineHandlesGroup.visible(false);
      return;
    }

    if (isSingleLineLike(this.#nodes)) {
      this.#rectHandlesGroup.visible(false);
      this.#lineHandlesGroup.visible(true);

      const node = this.#nodes[0];
      const { x, y, dx, dy } = node.attrs;

      const scale = this.#stage.scaleX();
      const endpointCircleSize = CORNER_RECT_SIZE / scale;
      const endpointCircleStrokeWidth = CORNER_RECT_STROKE_WIDTH / scale;

      this.#lineHandles.get('start')?.setAttrs({
        x,
        y,
        radius: endpointCircleSize / 2,
        strokeWidth: endpointCircleStrokeWidth,
      });

      this.#lineHandles.get('end')?.setAttrs({
        x: x + dx,
        y: y + dy,
        radius: endpointCircleSize / 2,
        strokeWidth: endpointCircleStrokeWidth,
      });

      return;
    }

    this.#rectHandlesGroup.visible(true);
    this.#lineHandlesGroup.visible(false);

    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;

    for (const node of this.#nodes) {
      const rect = node.getClientRect();

      minX = Math.min(minX, rect.x);
      minY = Math.min(minY, rect.y);
      maxX = Math.max(maxX, rect.x + rect.width);
      maxY = Math.max(maxY, rect.y + rect.height);
    }

    if (!Number.isFinite(minX) || !Number.isFinite(minY) || !Number.isFinite(maxX) || !Number.isFinite(maxY)) {
      this.#rectHandlesGroup.visible(false);
      return;
    }

    const scale = this.#stage.scaleX();
    const pos = this.#stage.position();

    const x = (minX - pos.x) / scale;
    const y = (minY - pos.y) / scale;
    const width = (maxX - minX) / scale;
    const height = (maxY - minY) / scale;

    const cornerRectSize = CORNER_RECT_SIZE / scale;
    const cornerRectStrokeWidth = CORNER_RECT_STROKE_WIDTH / scale;
    const edgeLineStrokeWidth = EDGE_LINE_STROKE_WIDTH / scale;

    this.#rectHandles.get('tl')?.setAttrs({
      x: x - cornerRectSize / 2,
      y: y - cornerRectSize / 2,
      width: cornerRectSize,
      height: cornerRectSize,
      strokeWidth: cornerRectStrokeWidth,
      cornerRadius: cornerRectSize / 4,
    });

    this.#rectHandles.get('tr')?.setAttrs({
      x: x + width - cornerRectSize / 2,
      y: y - cornerRectSize / 2,
      width: cornerRectSize,
      height: cornerRectSize,
      strokeWidth: cornerRectStrokeWidth,
      cornerRadius: cornerRectSize / 4,
    });

    this.#rectHandles.get('br')?.setAttrs({
      x: x + width - cornerRectSize / 2,
      y: y + height - cornerRectSize / 2,
      width: cornerRectSize,
      height: cornerRectSize,
      strokeWidth: cornerRectStrokeWidth,
      cornerRadius: cornerRectSize / 4,
    });

    this.#rectHandles.get('bl')?.setAttrs({
      x: x - cornerRectSize / 2,
      y: y + height - cornerRectSize / 2,
      width: cornerRectSize,
      height: cornerRectSize,
      strokeWidth: cornerRectStrokeWidth,
      cornerRadius: cornerRectSize / 4,
    });

    this.#rectHandles.get('t')?.setAttrs({
      points: [x, y, x + width, y],
      strokeWidth: edgeLineStrokeWidth,
      hitStrokeWidth: edgeLineStrokeWidth * 4,
    });

    this.#rectHandles.get('r')?.setAttrs({
      points: [x + width, y, x + width, y + height],
      strokeWidth: edgeLineStrokeWidth,
      hitStrokeWidth: edgeLineStrokeWidth * 4,
    });

    this.#rectHandles.get('b')?.setAttrs({
      points: [x + width, y + height, x, y + height],
      strokeWidth: edgeLineStrokeWidth,
      hitStrokeWidth: edgeLineStrokeWidth * 4,
    });

    this.#rectHandles.get('l')?.setAttrs({
      points: [x, y + height, x, y],
      strokeWidth: edgeLineStrokeWidth,
      hitStrokeWidth: edgeLineStrokeWidth * 4,
    });
  }

  handle(pos: Pos) {
    const intersection = this.#layer.getIntersection(pos);
    if (!intersection) {
      return null;
    }

    return intersection.attrs.handle as ResizeHandle;
  }

  isInsideBoundingBox(pos: Pos) {
    if (this.#nodes.length === 0) {
      return false;
    }

    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;

    for (const node of this.#nodes) {
      const rect = node.getClientRect();
      minX = Math.min(minX, rect.x);
      minY = Math.min(minY, rect.y);
      maxX = Math.max(maxX, rect.x + rect.width);
      maxY = Math.max(maxY, rect.y + rect.height);
    }

    const scale = this.#stage.scaleX();
    const stagePos = this.#stage.position();

    const stageX = (pos.x - stagePos.x) / scale;
    const stageY = (pos.y - stagePos.y) / scale;

    const boxMinX = (minX - stagePos.x) / scale;
    const boxMinY = (minY - stagePos.y) / scale;
    const boxMaxX = (maxX - stagePos.x) / scale;
    const boxMaxY = (maxY - stagePos.y) / scale;

    return stageX >= boxMinX && stageX <= boxMaxX && stageY >= boxMinY && stageY <= boxMaxY;
  }
}
