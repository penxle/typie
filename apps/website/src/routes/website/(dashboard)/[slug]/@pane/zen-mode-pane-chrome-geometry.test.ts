import { afterEach, describe, expect, it, vi } from 'vitest';
import { PaneChromeGeometry } from './zen-mode-pane-chrome-geometry';

const rect = (top: number, height: number): DOMRect =>
  ({ top, right: 600, bottom: top + height, left: 0, width: 600, height, x: 0, y: top }) as DOMRect;

describe('PaneChromeGeometry', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('keeps chrome insets stable when a resized pane moves', () => {
    const resize = new Map<Element, () => void>();
    class ResizeObserverStub {
      #callback: ResizeObserverCallback;
      #targets = new Set<Element>();

      constructor(callback: ResizeObserverCallback) {
        this.#callback = callback;
      }

      observe(target: Element): void {
        this.#targets.add(target);
        resize.set(target, () => this.#callback([], this as unknown as ResizeObserver));
      }

      disconnect(): void {
        for (const target of this.#targets) resize.delete(target);
        this.#targets.clear();
      }
    }
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);

    let rootTop = 500;
    let headerTop = 500;
    let toolbarTop = 537;
    const root = { getBoundingClientRect: () => rect(rootTop, 240) } as HTMLElement;
    const header = { getBoundingClientRect: () => rect(headerTop, 37) } as HTMLElement;
    const toolbar = { getBoundingClientRect: () => rect(toolbarTop, 41) } as HTMLElement;
    const geometry = new PaneChromeGeometry(vi.fn());

    geometry.registerRoot(root);
    geometry.registerHeaderLane(header);
    geometry.registerToolbarLane(toolbar);
    expect(geometry.floatingChromeInset()).toBe(78);
    expect(geometry.visibleChromeInset(true, true)).toBe(78);

    rootTop = 0;
    headerTop = 0;
    toolbarTop = 37;
    resize.get(root)?.();

    expect(geometry.floatingChromeInset()).toBe(78);
    expect(geometry.visibleChromeInset(true, true)).toBe(78);
  });

  it('uses the current lane position when a same-sized pane moves', () => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {
          // This case intentionally leaves the cached size unchanged.
        }
        disconnect(): void {
          // No observer resources in the stub.
        }
      },
    );

    let headerTop = 100;
    const header = { getBoundingClientRect: () => rect(headerTop, 36) } as HTMLElement;
    const geometry = new PaneChromeGeometry(vi.fn());
    geometry.registerHeaderLane(header);

    expect(geometry.pointInLane({ x: 20, y: 118 }, 'header')).toEqual({ x: 20, y: 18 });

    headerTop = 400;
    expect(geometry.pointInLane({ x: 20, y: 418 }, 'header')).toEqual({ x: 20, y: 18 });
  });
});
