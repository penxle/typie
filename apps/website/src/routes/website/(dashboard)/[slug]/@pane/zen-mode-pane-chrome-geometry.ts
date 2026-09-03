import type { ActionReturn } from 'svelte/action';

export type PaneChromeSegment = 'identity' | 'actions' | 'toolbar';
export type PaneChromeLane = 'header' | 'toolbar';
export type PaneChromeSegmentGeometry = {
  left: number;
  right: number;
  width: number;
};

type GeometryKey = PaneChromeSegment | 'root' | 'header';
type Point = { x: number; y: number };

const geometryKeys: readonly GeometryKey[] = ['root', 'header', 'identity', 'actions', 'toolbar'];

const contains = (rect: DOMRect | undefined, x: number, y: number): boolean =>
  rect !== undefined && x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;

export class PaneChromeGeometry {
  #nodes: Partial<Record<GeometryKey, HTMLElement>> = {};
  #rects: Partial<Record<GeometryKey, DOMRect>> = {};
  #observers = new Map<HTMLElement, ResizeObserver>();
  #onMeasure: () => void;

  constructor(onMeasure: () => void) {
    this.#onMeasure = onMeasure;
  }

  registerRoot(node: HTMLElement): ActionReturn {
    return this.#register('root', node);
  }

  registerHeaderLane(node: HTMLElement): ActionReturn {
    return this.#register('header', node);
  }

  registerToolbarLane(node: HTMLElement): ActionReturn {
    return this.#register('toolbar', node);
  }

  registerSegment(segment: Exclude<PaneChromeSegment, 'toolbar'>, node: HTMLElement): ActionReturn {
    return this.#register(segment, node);
  }

  zoneAt(x: number, y: number): PaneChromeSegment | 'gap' | null {
    const toolbar = this.#nodes.toolbar?.getBoundingClientRect();
    if (contains(toolbar, x, y)) return 'toolbar';

    const header = this.#nodes.header?.getBoundingClientRect();
    const identity = this.#nodes.identity?.getBoundingClientRect();
    const actions = this.#nodes.actions?.getBoundingClientRect();
    if (!header || !identity || !actions || y < header.top || y > header.bottom) return null;

    const identityRight = header.left + identity.width;
    const actionsLeft = header.right - actions.width;
    if (x <= identityRight) return 'identity';
    if (x >= actionsLeft) return 'actions';
    if (x > identityRight && x < actionsLeft) return 'gap';
    return null;
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- public geometry queries stay grouped before private implementation helpers
  #laneRect(lane: PaneChromeLane): DOMRect | undefined {
    return (lane === 'header' ? this.#nodes.header : this.#nodes.toolbar)?.getBoundingClientRect();
  }

  pointInLane(point: Point, lane: PaneChromeLane): Point | null {
    const rect = this.#laneRect(lane);
    return rect ? { x: point.x - rect.left, y: point.y - rect.top } : null;
  }

  laneSize(lane: PaneChromeLane): { width: number; height: number } {
    const rect = this.#laneRect(lane);
    return { width: rect?.width ?? 0, height: rect?.height ?? 0 };
  }

  segmentGeometry(segment: PaneChromeSegment): PaneChromeSegmentGeometry | undefined {
    if (segment === 'toolbar') {
      const toolbar = this.#rects.toolbar;
      return toolbar ? { left: 0, right: toolbar.width, width: toolbar.width } : undefined;
    }
    const header = this.#rects.header;
    const rect = this.#rects[segment];
    if (!header || !rect) return;
    if (segment === 'identity') return { left: 0, right: rect.width, width: rect.width };
    return { left: header.width - rect.width, right: header.width, width: rect.width };
  }

  headerInset(): number {
    const height = this.#rects.header?.height;
    return height === undefined ? 0 : Math.max(0, Math.round(height));
  }

  floatingChromeInset(): number {
    const root = this.#nodes.root?.getBoundingClientRect();
    if (!root) return 0;
    const chromeBottom =
      this.#nodes.toolbar?.getBoundingClientRect().bottom ?? this.#nodes.header?.getBoundingClientRect().bottom ?? root.top;
    return Math.max(0, Math.round(chromeBottom - root.top));
  }

  visibleChromeInset(includeHeader: boolean, includeToolbar: boolean): number {
    const root = this.#nodes.root?.getBoundingClientRect();
    if (!root) return 0;
    let bottom = root.top;
    if (includeHeader) bottom = Math.max(bottom, this.#nodes.header?.getBoundingClientRect().bottom ?? root.top);
    if (includeToolbar) bottom = Math.max(bottom, this.#nodes.toolbar?.getBoundingClientRect().bottom ?? root.top);
    return Math.max(0, Math.round(bottom - root.top));
  }

  #register(key: GeometryKey, node: HTMLElement): ActionReturn {
    const previous = this.#nodes[key];
    if (previous && previous !== node) this.#unobserve(previous);
    this.#nodes[key] = node;
    this.#observe(node);
    return {
      destroy: () => {
        this.#unobserve(node);
        if (this.#nodes[key] !== node) return;
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete -- action teardown removes its DOM registration
        delete this.#nodes[key];
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete -- action teardown removes its cached measurement
        delete this.#rects[key];
        this.#onMeasure();
      },
    };
  }

  #observe(node: HTMLElement): void {
    this.#measure();
    const observer = new ResizeObserver(() => this.#measure());
    observer.observe(node);
    this.#observers.set(node, observer);
  }

  #measure(): void {
    for (const key of geometryKeys) {
      const node = this.#nodes[key];
      if (node) this.#rects[key] = node.getBoundingClientRect();
    }
    this.#onMeasure();
  }

  #unobserve(node: HTMLElement): void {
    this.#observers.get(node)?.disconnect();
    this.#observers.delete(node);
  }
}
