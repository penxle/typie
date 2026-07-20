import { createDragScroll, handleDragScroll } from '@typie/ui/utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ScrollViewport } from '@typie/ui/utils';

const createViewport = () => {
  const target = document.createElement('div');
  let scrollTop = 0;
  let scrollLeft = 0;

  const viewport: ScrollViewport = {
    target,
    getRect: () => ({ top: 0, bottom: 100, left: 0, right: 100 }),
    getScrollTop: () => scrollTop,
    getScrollLeft: () => scrollLeft,
    getScrollHeight: () => 1000,
    scrollBy: (x, y) => {
      scrollLeft += x;
      scrollTop += y;
    },
    scrollTo: (options) => {
      if (options.top !== undefined) scrollTop = options.top;
      if (options.left !== undefined) scrollLeft = options.left;
    },
  };

  return { target, viewport };
};

const installAnimationFrames = () => {
  let nextId = 1;
  let now = 0;
  const frames = new Map<number, FrameRequestCallback>();

  vi.stubGlobal(
    'requestAnimationFrame',
    vi.fn((callback: FrameRequestCallback) => {
      const id = nextId++;
      frames.set(id, callback);
      return id;
    }),
  );
  vi.stubGlobal(
    'cancelAnimationFrame',
    vi.fn((id: number) => {
      frames.delete(id);
    }),
  );
  vi.spyOn(performance, 'now').mockImplementation(() => now);

  return {
    pendingCount: () => frames.size,
    runNext: (time: number) => {
      const frame = frames.entries().next().value as [number, FrameRequestCallback] | undefined;
      expect(frame).toBeDefined();
      if (!frame) return;

      const [id, callback] = frame;
      frames.delete(id);
      now = time;
      callback(time);
    },
  };
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('drag scroll', () => {
  it('keeps one throttle clock while the owner updates the pointer', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const onScroll = vi.fn();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      scrollZoneSize: 20,
      onScrollThrottleMs: 50,
      initialPointer: { clientX: 50, clientY: 95 },
      onScroll,
    });

    frames.runNext(100);
    expect(onScroll).toHaveBeenCalledTimes(1);
    expect(onScroll).toHaveBeenLastCalledWith(50, 95);

    dragScroll.updatePointer(50, 90);
    frames.runNext(120);
    expect(onScroll).toHaveBeenCalledTimes(1);

    dragScroll.updatePointer(50, 99);
    frames.runNext(160);
    expect(onScroll).toHaveBeenCalledTimes(2);
    expect(onScroll).toHaveBeenLastCalledWith(50, 99);

    dragScroll.destroy();
  });

  it('does not revive after destroy runs inside onScroll', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      scrollZoneSize: 20,
      onScrollThrottleMs: 50,
      onScroll: () => dragScroll.destroy(),
    });

    dragScroll.updatePointer(50, 95);
    frames.runNext(100);
    expect(frames.pendingCount()).toBe(0);

    dragScroll.updatePointer(50, 99);
    expect(frames.pendingCount()).toBe(0);
  });

  it('keeps forwarding viewport pointer moves until cleanup', () => {
    const { target, viewport } = createViewport();
    const frames = installAnimationFrames();
    const onScroll = vi.fn();
    const cleanup = handleDragScroll(viewport, true, {
      stickyCandidates: [],
      scrollZoneSize: 20,
      onScrollThrottleMs: 50,
      onScroll,
    });

    target.dispatchEvent(new MouseEvent('pointermove', { clientX: 50, clientY: 95 }));
    frames.runNext(100);
    expect(onScroll).toHaveBeenCalledWith(50, 95);

    cleanup?.();
    expect(frames.pendingCount()).toBe(0);

    target.dispatchEvent(new MouseEvent('pointermove', { clientX: 50, clientY: 99 }));
    expect(frames.pendingCount()).toBe(0);
  });
});
