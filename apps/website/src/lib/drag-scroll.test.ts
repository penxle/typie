import { createDragScroll, handleDragScroll } from '@typie/ui/utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ScrollViewport } from '@typie/ui/utils';

type ViewportRect = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

const DEFAULT_VIEWPORT_RECT = { top: 0, bottom: 200, left: 0, right: 200 };

const createViewport = (rect: ViewportRect = DEFAULT_VIEWPORT_RECT) => {
  const target = document.createElement('div');
  let scrollTop = 0;
  let scrollLeft = 0;

  const viewport: ScrollViewport = {
    target,
    getRect: () => rect,
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

  return {
    target,
    viewport,
    getScrollTop: () => scrollTop,
    getScrollLeft: () => scrollLeft,
  };
};

const installAnimationFrames = () => {
  let nextId = 1;
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

  return {
    pendingCount: () => frames.size,
    runNext: (time: number) => {
      const frame = frames.entries().next().value as [number, FrameRequestCallback] | undefined;
      expect(frame).toBeDefined();
      if (!frame) return;

      const [id, callback] = frame;
      frames.delete(id);
      callback(time);
    },
  };
};

const createStickyCandidate = (rect: ViewportRect) => {
  const element = document.createElement('div');
  vi.spyOn(element, 'getBoundingClientRect').mockReturnValue(rect as DOMRect);
  return element;
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('drag scroll', () => {
  it('does not start at the shared zone boundary', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      initialPointer: { clientX: 100, clientY: 60 },
    });

    expect(frames.pendingCount()).toBe(0);

    dragScroll.destroy();
  });

  it.each([
    { pointerY: 59.999, expectedDelta: -24 },
    { pointerY: 30, expectedDelta: -60 },
    { pointerY: 0, expectedDelta: -96 },
    { pointerY: -10, expectedDelta: -126 },
    { pointerY: -20, expectedDelta: -156 },
    { pointerY: -28, expectedDelta: -180 },
    { pointerY: -100, expectedDelta: -180 },
  ])('scrolls $expectedDelta px over 100 ms when top-edge pointer y is $pointerY', ({ pointerY, expectedDelta }) => {
    const { viewport, getScrollTop } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      initialPointer: { clientX: 100, clientY: pointerY },
    });

    frames.runNext(0);
    expect(getScrollTop()).toBe(0);
    frames.runNext(100);
    expect(getScrollTop()).toBeCloseTo(expectedDelta, 1);

    dragScroll.destroy();
  });

  it('applies the outside curve symmetrically at the bottom edge', () => {
    const { viewport, getScrollTop } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      initialPointer: { clientX: 100, clientY: 220 },
    });

    frames.runNext(0);
    frames.runNext(100);
    expect(getScrollTop()).toBeCloseTo(156, 5);

    dragScroll.destroy();
  });

  it('computes horizontal and vertical speeds independently', () => {
    const { viewport, getScrollLeft, getScrollTop } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      axis: 'both',
      stickyCandidates: [],
      initialPointer: { clientX: -20, clientY: -10 },
    });

    frames.runNext(0);
    frames.runNext(100);
    expect(getScrollLeft()).toBeCloseTo(-156, 5);
    expect(getScrollTop()).toBeCloseTo(-126, 5);

    dragScroll.destroy();
  });

  it('scrolls the same distance over one second at 60 Hz and 120 Hz', () => {
    const frames = installAnimationFrames();

    const runAtRefreshRate = (startTime: number, frameCount: number) => {
      const { viewport, getScrollTop } = createViewport();
      const dragScroll = createDragScroll(viewport, {
        stickyCandidates: [],
        initialPointer: { clientX: 100, clientY: 0 },
      });

      frames.runNext(startTime);
      for (let frame = 1; frame <= frameCount; frame++) {
        frames.runNext(startTime + (frame * 1000) / frameCount);
      }
      const scrollTop = getScrollTop();
      dragScroll.destroy();
      return scrollTop;
    };

    const at60Hz = runAtRefreshRate(0, 60);
    const at120Hz = runAtRefreshRate(2000, 120);

    expect(at60Hz).toBeCloseTo(-960, 5);
    expect(at120Hz).toBeCloseTo(-960, 5);
    expect(at120Hz).toBeCloseTo(at60Hz, 5);
  });

  it('seeds a fresh frame clock after the loop stops and restarts', () => {
    const { viewport, getScrollTop } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      initialPointer: { clientX: 100, clientY: 0 },
    });

    frames.runNext(10);
    expect(getScrollTop()).toBe(0);

    dragScroll.updatePointer(100, 100);
    frames.runNext(20);
    expect(frames.pendingCount()).toBe(0);

    dragScroll.updatePointer(100, 0);
    frames.runNext(1000);
    expect(getScrollTop()).toBe(0);
    frames.runNext(1100);
    expect(getScrollTop()).toBeCloseTo(-96, 5);

    dragScroll.destroy();
  });

  it('uses the 120 px zone-derived sticky candidate range', () => {
    const { viewport, getScrollTop } = createViewport({ top: 0, bottom: 300, left: 0, right: 200 });
    const frames = installAnimationFrames();
    const inRange = createStickyCandidate({ top: 120, bottom: 140, left: 0, right: 200 });
    const withInRangeCandidate = createDragScroll(viewport, {
      stickyCandidates: [inRange],
      initialPointer: { clientX: 100, clientY: 150 },
    });

    frames.runNext(0);
    frames.runNext(100);
    expect(getScrollTop()).toBeCloseTo(-84, 5);
    withInRangeCandidate.destroy();

    const beyondRange = createStickyCandidate({ top: 120.001, bottom: 140.001, left: 0, right: 200 });
    const withBeyondRangeCandidate = createDragScroll(viewport, {
      stickyCandidates: [beyondRange],
      initialPointer: { clientX: 100, clientY: 150 },
    });

    expect(frames.pendingCount()).toBe(0);
    expect(getScrollTop()).toBeCloseTo(-84, 5);

    withBeyondRangeCandidate.destroy();
  });

  it('keeps one throttle clock while the owner updates the pointer', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const onScroll = vi.fn();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      onScrollThrottleMs: 50,
      initialPointer: { clientX: 100, clientY: 0 },
      onScroll,
    });

    frames.runNext(100);
    expect(onScroll).not.toHaveBeenCalled();
    frames.runNext(120);
    expect(onScroll).toHaveBeenCalledTimes(1);
    expect(onScroll).toHaveBeenLastCalledWith(100, 0, expect.any(Object));

    dragScroll.updatePointer(100, 170);
    frames.runNext(140);
    expect(onScroll).toHaveBeenCalledTimes(1);

    dragScroll.updatePointer(100, 199);
    frames.runNext(180);
    expect(onScroll).toHaveBeenCalledTimes(2);
    expect(onScroll).toHaveBeenLastCalledWith(100, 199, expect.any(Object));

    dragScroll.destroy();
  });

  it('reports the actual scroll delta after reversing edge direction', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const onScroll = vi.fn();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      onScrollThrottleMs: 0,
      initialPointer: { clientX: 100, clientY: 199 },
      onScroll,
    });

    frames.runNext(0);
    frames.runNext(100);
    expect(onScroll).toHaveBeenLastCalledWith(100, 199, expect.objectContaining({ deltaX: 0, deltaY: expect.any(Number) }));
    expect(onScroll.mock.lastCall?.[2]?.deltaY).toBeGreaterThan(0);

    dragScroll.updatePointer(100, 0);
    frames.runNext(200);
    expect(onScroll).toHaveBeenLastCalledWith(100, 0, expect.objectContaining({ deltaX: 0, deltaY: expect.any(Number) }));
    expect(onScroll.mock.lastCall?.[2]?.deltaY).toBeLessThan(0);

    dragScroll.destroy();
  });

  it('does not revive after destroy runs inside onScroll', () => {
    const { viewport } = createViewport();
    const frames = installAnimationFrames();
    const dragScroll = createDragScroll(viewport, {
      stickyCandidates: [],
      onScrollThrottleMs: 50,
      onScroll: () => dragScroll.destroy(),
    });

    dragScroll.updatePointer(100, 0);
    frames.runNext(0);
    frames.runNext(100);
    expect(frames.pendingCount()).toBe(0);

    dragScroll.updatePointer(100, -20);
    expect(frames.pendingCount()).toBe(0);
  });

  it('keeps forwarding viewport pointer moves until cleanup', () => {
    const { target, viewport } = createViewport();
    const frames = installAnimationFrames();
    const onScroll = vi.fn();
    const cleanup = handleDragScroll(viewport, true, {
      stickyCandidates: [],
      onScrollThrottleMs: 50,
      onScroll,
    });

    target.dispatchEvent(new MouseEvent('pointermove', { clientX: 100, clientY: 0 }));
    frames.runNext(0);
    frames.runNext(100);
    expect(onScroll).toHaveBeenCalledWith(100, 0, expect.any(Object));

    cleanup?.();
    expect(frames.pendingCount()).toBe(0);

    target.dispatchEvent(new MouseEvent('pointermove', { clientX: 100, clientY: -20 }));
    expect(frames.pendingCount()).toBe(0);
  });
});
