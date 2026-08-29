import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  RENDER_ZOOM_MAX_COMMIT_DELAY_MS,
  RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS,
  RENDER_ZOOM_SCALE_RATIO_THRESHOLD,
} from '$lib/editor-ffi/zoom';
import { EditorZoomController } from './editor-zoom.svelte';
import type { ScrollViewport } from '@typie/ui/utils';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
import type { DocumentZoomLayout } from '$lib/editor-ffi/zoom';

type WheelEventDouble = WheelEvent & {
  preventDefault: ReturnType<typeof vi.fn>;
};

const controllers: EditorZoomController[] = [];
const defaultLayout: DocumentZoomLayout = { type: 'paginated', pageWidth: 1000 };

const createController = (layout: DocumentZoomLayout = defaultLayout): EditorZoomController => {
  const editor = {
    clientToLocal: vi.fn(() => null),
    pageSizes: [],
    pageEls: [],
  } as unknown as Editor;
  const controller = new EditorZoomController({
    editor,
    layout: () => layout,
    viewportWidth: () => 1000,
    getScrollViewport: () => null,
  });
  controllers.push(controller);
  return controller;
};

describe('EditorZoomController continuous timing', () => {
  it('starts continuous layout at unit zoom', () => {
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.syncInitialZoom();

    expect(controller.displayZoom).toBe(1);
    expect(controller.renderZoom).toBe(1);
  });

  it('commits a small continuous render zoom change at the existing debounce boundary', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(0.9);
    expect(controller.displayZoom).toBe(0.9);
    expect(controller.renderZoom).toBe(1);

    vi.advanceTimersByTime(119);
    expect(controller.renderZoom).toBe(1);

    vi.advanceTimersByTime(1);
    expect(controller.renderZoom).toBe(0.9);
  });

  it('commits continuous render zoom immediately at gesture end', async () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 0.9, 0, 0, 16);
    await controller.releaseDirectZoom('touch');

    expect(controller.renderZoom).toBe(0.9);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('commits a large scale difference without waiting for the quiet period', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(1 / RENDER_ZOOM_SCALE_RATIO_THRESHOLD);

    expect(controller.renderZoom).toBeCloseTo(1 / RENDER_ZOOM_SCALE_RATIO_THRESHOLD);
  });

  it('keeps the minimum interval while coalescing rapid large scale changes', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(1 / RENDER_ZOOM_SCALE_RATIO_THRESHOLD);
    const firstRenderZoom = controller.renderZoom;
    controller.setZoom(1.2);
    vi.advanceTimersByTime(RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS - 1);
    controller.setZoom(1.3);

    expect(controller.renderZoom).toBe(firstRenderZoom);

    vi.advanceTimersByTime(1);
    expect(controller.renderZoom).toBe(1.3);
  });

  it('replaces a blocked threshold commit with the latest input without extending the maximum delay', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(0.84);
    vi.advanceTimersByTime(20);
    controller.setZoom(1.2);
    vi.advanceTimersByTime(80);
    controller.setZoom(0.9);
    vi.advanceTimersByTime(100);
    controller.setZoom(0.91);
    vi.advanceTimersByTime(100);
    controller.setZoom(0.9);
    vi.advanceTimersByTime(19);

    expect(controller.renderZoom).toBe(0.84);

    vi.advanceTimersByTime(1);
    expect(controller.renderZoom).toBe(0.9);
  });

  it('commits continuous input by the maximum delay even when the quiet period keeps moving', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(0.95);
    vi.advanceTimersByTime(100);
    controller.setZoom(0.96);
    vi.advanceTimersByTime(100);
    controller.setZoom(0.95);
    vi.advanceTimersByTime(RENDER_ZOOM_MAX_COMMIT_DELAY_MS - 201);

    expect(controller.renderZoom).toBe(1);

    vi.advanceTimersByTime(1);
    expect(controller.renderZoom).toBe(0.95);
  });

  it('defers a gesture-end commit until the minimum interval after an intermediate commit', async () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(1 / RENDER_ZOOM_SCALE_RATIO_THRESHOLD);
    const firstRenderZoom = controller.renderZoom;
    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 1.1, 0, 0, 16);
    await controller.releaseDirectZoom('touch');

    expect(controller.renderZoom).toBe(firstRenderZoom);

    vi.advanceTimersByTime(RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS);
    expect(controller.renderZoom).toBe(1.1);
  });
});

describe('EditorZoomController indicator toggle', () => {
  const createToggleController = (viewportWidth: number): EditorZoomController => {
    const editor = {
      clientToLocal: vi.fn(() => null),
      pageSizes: [],
      pageEls: [],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => defaultLayout,
      viewportWidth: () => viewportWidth,
      getScrollViewport: () => null,
    });
    controllers.push(controller);
    return controller;
  };

  it('toggles fit-width to unit and unit to fit-width', async () => {
    const controller = createToggleController(500);
    controller.syncInitialZoom();

    expect(controller.displayZoom).toBe(0.5);
    await expect(controller.toggleZoomByIndicator()).resolves.toBe(true);
    expect(controller.displayZoom).toBe(1);

    await expect(controller.toggleZoomByIndicator()).resolves.toBe(true);
    expect(controller.displayZoom).toBe(0.5);
  });

  it('returns any unnamed zoom to unit', async () => {
    const controller = createToggleController(500);
    controller.setZoom(1.4, { commitRender: true });

    await expect(controller.toggleZoomByIndicator()).resolves.toBe(true);
    expect(controller.displayZoom).toBe(1);
  });

  it('does nothing when unit and fit-width are the same zoom', async () => {
    const controller = createToggleController(1000);
    controller.syncInitialZoom();

    await expect(controller.toggleZoomByIndicator()).resolves.toBe(false);
    expect(controller.displayZoom).toBe(1);
  });

  it('reports a bound when the requested fit zoom is clamped', async () => {
    const controller = createToggleController(2500);
    controller.syncInitialZoom();
    controller.setZoom(1, { commitRender: true });

    await expect(controller.toggleZoomByIndicator()).resolves.toBe(true);
    expect(controller.displayZoom).toBe(2);
  });

  it('reuses one viewport anchor for rapid toggles before layout catches up', async () => {
    let scrollTop = 300;
    const scrollBy = vi.fn((_deltaX: number, deltaY: number) => {
      scrollTop += deltaY;
    });
    const viewport = {
      getRect: () => ({ left: 0, top: 0, right: 500, bottom: 500 }),
      getScrollLeft: () => 0,
      getScrollTop: () => scrollTop,
      scrollBy,
    };
    const clientToLocal = vi.fn().mockReturnValueOnce({ page: 0, x: 250, y: 550 }).mockReturnValue({ page: 0, x: 250, y: 900 });
    const beginViewportZoom = vi.fn();
    const updateViewportZoomAttachment = vi.fn();
    const editor = {
      clientToLocal,
      pageSizes: [{ width: 1000, height: 2000 }],
      pageEls: [{ getBoundingClientRect: () => ({ left: 0, top: -scrollTop }) }],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => defaultLayout,
      viewportWidth: () => 500,
      getScrollViewport: () => viewport as unknown as ScrollViewport,
      beginViewportZoom,
      updateViewportZoomAttachment,
    });
    controllers.push(controller);

    const fit = controller.toggleZoomByIndicator();
    const unit = controller.toggleZoomByIndicator();

    expect(clientToLocal).toHaveBeenCalledOnce();
    await Promise.all([fit, unit]);
    expect(beginViewportZoom).toHaveBeenCalledOnce();
    expect(updateViewportZoomAttachment).toHaveBeenCalledOnce();
  });

  it('keeps settled indicator toggles reversible by retaining the scroll owner viewport anchor', async () => {
    let scrollTop = 300;
    const controllerRef: { current?: EditorZoomController } = {};
    let activeAnchor = { page: 0, x: 250, y: 550, focalX: 250, focalY: 250 };
    const viewport = {
      getRect: () => ({ left: 0, top: 0, right: 500, bottom: 500 }),
      getScrollLeft: () => 0,
      getScrollTop: () => scrollTop,
      scrollBy: (_deltaX: number, deltaY: number) => {
        scrollTop += deltaY;
      },
    };
    const clientToLocal = vi.fn(() => ({
      page: 0,
      x: 250,
      y: (scrollTop + 250) / (controllerRef.current?.displayZoom ?? 1) + 0.25,
    }));
    const editor = {
      clientToLocal,
      pageSizes: [{ width: 1000, height: 2000 }],
      pageEls: [{ getBoundingClientRect: () => ({ left: 0, top: -scrollTop }) }],
    } as unknown as Editor;
    const beginViewportZoom = vi.fn((point: { page: number; x: number; y: number }) => {
      activeAnchor = { ...point, focalX: 250, focalY: 250 };
    });
    const controller = new EditorZoomController({
      editor,
      layout: () => defaultLayout,
      viewportWidth: () => 500,
      getScrollViewport: () => viewport as unknown as ScrollViewport,
      beginViewportZoom,
      resolveViewportAnchor: () => activeAnchor,
    });
    controllerRef.current = controller;
    controllers.push(controller);

    for (let cycle = 0; cycle < 4; cycle += 1) {
      await controller.toggleZoomByIndicator();
      await controller.toggleZoomByIndicator();
    }

    expect(clientToLocal).not.toHaveBeenCalled();
    expect(beginViewportZoom).not.toHaveBeenCalled();
    expect(scrollTop).toBeCloseTo(300);
  });

  it('does not resnap a settled zoom when the viewport resizes', async () => {
    let viewportWidth = 500;
    const editor = {
      clientToLocal: vi.fn(() => null),
      pageSizes: [],
      pageEls: [],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => ({ type: 'continuous', maxWidth: 600 }),
      viewportWidth: () => viewportWidth,
      getScrollViewport: () => null,
    });
    controllers.push(controller);
    controller.syncInitialZoom();

    controller.setZoom(0.79, { commitRender: true });
    expect(controller.displayZoom).toBe(0.78125);
    expect(controller.landmark).toBe('fit-width');

    viewportWidth = 480;
    controller.clampCurrentZoomToBounds();

    expect(controller.displayZoom).toBe(0.78125);
    expect(controller.landmark).toBeNull();
  });
});

const wheelEvent = ({
  metaKey = false,
  ctrlKey = false,
  deltaX = 0,
  deltaY = -20,
  timeStamp = 0,
  cancelable = true,
}: {
  metaKey?: boolean;
  ctrlKey?: boolean;
  deltaX?: number;
  deltaY?: number;
  timeStamp?: number;
  cancelable?: boolean;
} = {}): WheelEventDouble =>
  ({
    metaKey,
    ctrlKey,
    deltaX,
    deltaY,
    timeStamp,
    cancelable,
    clientX: 0,
    clientY: 0,
    preventDefault: vi.fn(),
  }) as unknown as WheelEventDouble;

afterEach(() => {
  for (const controller of controllers) {
    controller.destroy();
  }
  controllers.length = 0;
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('EditorZoomController keyboard steps', () => {
  it('moves to the next ten-percent grid line instead of adding ten percent', async () => {
    const controller = createController();

    controller.setZoom(0.53, { commitRender: true });
    expect(await controller.zoomInByKeyboard()).toBe(true);
    expect(controller.displayZoom).toBeCloseTo(0.6);

    controller.setZoom(0.53, { commitRender: true });
    expect(await controller.zoomOutByKeyboard()).toBe(true);
    expect(controller.displayZoom).toBeCloseTo(0.5);
  });

  it('stops at intervening landmarks as well as ten-percent grid lines', async () => {
    const layout = { type: 'paginated', pageWidth: 1000 } as const;
    const editor = {
      clientToLocal: vi.fn(() => null),
      pageSizes: [],
      pageEls: [],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => layout,
      viewportWidth: () => 520,
      getScrollViewport: () => null,
    });
    controllers.push(controller);

    controller.setZoom(0.45, { commitRender: true });
    await controller.zoomInByKeyboard();
    expect(controller.displayZoom).toBeCloseTo(0.5);
    await controller.zoomInByKeyboard();
    expect(controller.displayZoom).toBeCloseTo(0.52);

    controller.setZoom(0.58, { commitRender: true });
    await controller.zoomOutByKeyboard();
    expect(controller.displayZoom).toBeCloseTo(0.52);
  });

  it('reports whether a minimum or maximum step changed the applied zoom', async () => {
    const controller = createController();

    controller.setZoom(0.1, { commitRender: true });
    expect(await controller.zoomOutByKeyboard()).toBe(false);
    expect(await controller.zoomInByKeyboard()).toBe(true);

    controller.setZoom(2, { commitRender: true });
    expect(await controller.zoomInByKeyboard()).toBe(false);
    expect(await controller.zoomOutByKeyboard()).toBe(true);
  });

  it('keeps an outward keyboard step unavailable during elastic overshoot', async () => {
    const controller = createController();

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 0.05, 0, 0, 250);
    const overshootZoom = controller.displayZoom;
    expect(overshootZoom).toBeLessThan(0.1);

    expect(await controller.zoomOutByKeyboard()).toBe(false);
    expect(controller.displayZoom).toBe(overshootZoom);
  });
});

describe('EditorZoomController direct zoom detents', () => {
  it('snaps slow direct input while keeping the raw gesture free to continue', async () => {
    const controller = createController();
    controller.setZoom(0.95, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 0.99, 0, 0, 250);

    expect(controller.displayZoom).toBe(1);

    await controller.updateDirectZoom('touch', 0.97, 0, 0, 500);

    expect(controller.displayZoom).toBe(0.97);
  });

  it('lets fast direct input cross a snap point without capture', async () => {
    const controller = createController();
    controller.setZoom(0.95, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 0.99, 0, 0, 50);

    expect(controller.displayZoom).toBe(0.99);
  });

  it('does not assume the first wheel event is slow without a timing baseline', async () => {
    const controller = createController();
    controller.setZoom(0.95, { commitRender: true });

    await controller.handleWheel(
      wheelEvent({
        metaKey: true,
        deltaY: -240 * Math.log(0.99 / 0.95),
        timeStamp: 0,
      }),
    );

    expect(controller.displayZoom).toBeCloseTo(0.99);
  });

  it('stays settled when direct input is released on a detent', async () => {
    const controller = createController();
    controller.setZoom(0.95, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 0.99, 0, 0, 250);
    await controller.releaseDirectZoom('touch');

    expect(controller.displayZoom).toBe(1);
    expect(controller.hasActiveMotion).toBe(false);
  });

  it('keeps overzoom elastic when the fit detent equals the hard bound', async () => {
    const controller = createController({ type: 'continuous', maxWidth: 400 });
    controller.setZoom(2, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 2.2, 0, 0, 250);

    expect(controller.displayZoom).toBeGreaterThan(2);
    expect(controller.displayZoom).toBeLessThan(2.2);
  });

  it('settles overzoom immediately when reduced motion is requested', async () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn(() => ({ matches: true })),
    );
    const controller = createController({ type: 'continuous', maxWidth: 400 });
    controller.setZoom(2, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 2.2, 0, 0, 16);
    expect(controller.displayZoom).toBeGreaterThan(2);

    await controller.releaseDirectZoom('touch');

    expect(controller.displayZoom).toBe(2);
    expect(controller.hasActiveMotion).toBe(false);
  });

  it('settles tiny overzoom at the exact bound', async () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 400 });
    controller.setZoom(2, { commitRender: true });

    expect(controller.beginDirectZoom('touch', 0, 0, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 2.001, 0, 0, 250);
    expect(controller.displayZoom).toBeGreaterThan(2);

    await controller.releaseDirectZoom('touch');
    await vi.advanceTimersByTimeAsync(1000);

    expect(controller.displayZoom).toBe(2);
    expect(controller.hasActiveMotion).toBe(false);
  });

  it('stops at the released in-range zoom', async () => {
    vi.useFakeTimers();
    const scrollBy = vi.fn();
    const viewport = {
      getRect: () => ({ left: 0, top: 0, right: 1000, bottom: 1000 }),
      getScrollLeft: () => 0,
      getScrollTop: () => 0,
      scrollBy,
    };
    const editor = {
      clientToLocal: vi.fn(() => ({ page: 0, x: 100, y: 100 })),
      pageSizes: [{ width: 1000, height: 1000 }],
      pageEls: [{ getBoundingClientRect: () => ({ left: 0, top: 0 }) }],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => defaultLayout,
      viewportWidth: () => 1000,
      getScrollViewport: () => viewport as unknown as ScrollViewport,
    });
    controllers.push(controller);

    expect(controller.beginDirectZoom('touch', 100, 100, 0)).toBe(true);
    await controller.updateDirectZoom('touch', 1.1, 120, 100, 16);
    await controller.updateDirectZoom('touch', 1.2, 140, 100, 32);
    const scrollCallsAtRelease = scrollBy.mock.calls.length;
    await controller.releaseDirectZoom('touch');
    await vi.advanceTimersByTimeAsync(17);

    expect(controller.displayZoom).toBe(1.2);
    expect(controller.hasActiveMotion).toBe(false);
    expect(scrollBy).toHaveBeenCalledTimes(scrollCallsAtRelease);
  });

  it('drops a queued zoom anchor as soon as direct pan interrupts', async () => {
    const scrollBy = vi.fn();
    const viewport = {
      getRect: () => ({ left: 0, top: 0, right: 1000, bottom: 1000 }),
      getScrollLeft: () => 0,
      getScrollTop: () => 0,
      scrollBy,
    };
    const editor = {
      clientToLocal: vi.fn(() => ({ page: 0, x: 100, y: 100 })),
      pageSizes: [{ width: 1000, height: 1000 }],
      pageEls: [{ getBoundingClientRect: () => ({ left: 0, top: 0 }) }],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => defaultLayout,
      viewportWidth: () => 1000,
      getScrollViewport: () => viewport as unknown as ScrollViewport,
    });
    controllers.push(controller);

    expect(controller.beginDirectZoom('touch', 100, 100, 0)).toBe(true);
    const pendingUpdate = controller.updateDirectZoom('touch', 1.2, 100, 100, 16);
    controller.interruptForDirectPan();
    await pendingUpdate;

    expect(scrollBy).not.toHaveBeenCalled();
  });
});

describe('EditorZoomController.handleWheel', () => {
  it.each([
    ['Meta', { metaKey: true }],
    ['Control', { ctrlKey: true }],
  ])('zooms from the current event when %s is pressed', async (_, modifiers) => {
    const controller = createController();
    const event = wheelEvent(modifiers);

    await controller.handleWheel(event);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(controller.displayZoom).toBeGreaterThan(1);
  });

  it('leaves an unmodified wheel event to native scrolling', async () => {
    const controller = createController();
    const event = wheelEvent();

    await controller.handleWheel(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(controller.displayZoom).toBe(1);
  });

  it('does not retain the zoom anchor when an unmodified wheel starts panning', async () => {
    vi.useFakeTimers();
    const scrollBy = vi.fn();
    const viewport = {
      getRect: () => ({ left: 0, top: 0, right: 1000, bottom: 1000 }),
      getScrollLeft: () => 0,
      getScrollTop: () => 0,
      scrollBy,
    };
    const editor = {
      clientToLocal: vi.fn(() => ({ page: 0, x: 100, y: 100 })),
      pageSizes: [{ width: 1000, height: 1000 }],
      pageEls: [{ getBoundingClientRect: () => ({ left: 0, top: 0 }) }],
    } as unknown as Editor;
    const controller = new EditorZoomController({
      editor,
      layout: () => ({ type: 'continuous', maxWidth: 400 }),
      viewportWidth: () => 1000,
      getScrollViewport: () => viewport as unknown as ScrollViewport,
    });
    controllers.push(controller);
    controller.setZoom(2, { commitRender: true });

    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -20 }));
    const scrollCallsBeforePan = scrollBy.mock.calls.length;
    await controller.handleWheel(wheelEvent({ deltaY: 20, timeStamp: 16 }));
    await vi.advanceTimersByTimeAsync(1000);

    expect(controller.displayZoom).toBe(2);
    expect(scrollBy).toHaveBeenCalledTimes(scrollCallsBeforePan);
  });

  it.each([
    ['short gap and tiny delta', 8, -1],
    ['short gap and large delta', 8, -20],
    ['long gap and tiny delta', 1000, -1],
    ['long gap and large delta', 1000, -20],
  ])('leaves an unmodified event to scrolling after modified wheel input with a %s', async (_, timeStamp, deltaY) => {
    const controller = createController();
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -20 }));
    const zoomBeforeUnmodifiedEvent = controller.displayZoom;
    const event = wheelEvent({ deltaY, timeStamp });

    await controller.handleWheel(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(controller.displayZoom).toBe(zoomBeforeUnmodifiedEvent);
  });

  it.each([
    ['short gap and tiny delta', 8, -1],
    ['short gap and large delta', 8, -20],
    ['long gap and tiny delta', 1000, -1],
    ['long gap and large delta', 1000, -20],
  ])('routes a modified event to zoom after unmodified wheel input with a %s', async (_, timeStamp, deltaY) => {
    const controller = createController();
    await controller.handleWheel(wheelEvent({ deltaY: 20 }));
    const event = wheelEvent({ metaKey: true, deltaY, timeStamp });

    await controller.handleWheel(event);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    if (Math.abs(deltaY) > 1) {
      expect(controller.displayZoom).toBeGreaterThan(1);
    }
  });

  it('accumulates tiny modified wheel deltas before the quiet timeout', async () => {
    vi.useFakeTimers();
    const controller = createController();

    for (let index = 0; index < 5; index += 1) {
      await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -1, timeStamp: index * 8 }));
    }

    expect(controller.displayZoom).toBeGreaterThan(1);
  });

  it('commits the final render zoom after wheel input settles', async () => {
    vi.useFakeTimers();
    const controller = createController();

    for (let index = 0; index < 6; index += 1) {
      await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: index < 4 ? 8 : -8, timeStamp: index * 16 }));
    }
    await vi.advanceTimersByTimeAsync(1000);

    expect(controller.renderZoom).toBeCloseTo(controller.displayZoom);
    expect(controller.hasActiveMotion).toBe(false);
  });

  it('settles a tiny wheel burst before the next burst starts', async () => {
    vi.useFakeTimers();
    const controller = createController();

    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    expect(controller.hasActiveDirectZoom).toBe(true);
    await vi.advanceTimersByTimeAsync(400);
    expect(controller.displayZoom).toBe(1);
    expect(controller.hasActiveDirectZoom).toBe(false);

    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3, timeStamp: EditorZoomController.WHEEL_RAW_ZOOM_RESET_MS }));

    expect(controller.displayZoom).toBe(1);
    expect(controller.hasActiveDirectZoom).toBe(true);
  });

  it('cancels the raw wheel zoom reset when zoom is set programmatically', async () => {
    vi.useFakeTimers();
    const controller = createController();
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    expect(vi.getTimerCount()).toBeGreaterThanOrEqual(1);

    controller.setZoom(1, { commitRender: true });

    expect(vi.getTimerCount()).toBe(0);
  });

  it('cancels the raw wheel zoom reset when destroyed', async () => {
    vi.useFakeTimers();
    const controller = createController();
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    expect(vi.getTimerCount()).toBeGreaterThanOrEqual(1);

    controller.destroy();

    expect(vi.getTimerCount()).toBe(0);
  });

  it('ignores a zero-delta modified event', async () => {
    vi.useFakeTimers();
    const controller = createController();
    const event = wheelEvent({ metaKey: true, deltaY: 0 });

    await controller.handleWheel(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(controller.displayZoom).toBe(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('consumes a modified event at the maximum zoom bound', async () => {
    const controller = createController();
    controller.setZoom(2, { commitRender: true });
    const event = wheelEvent({ metaKey: true });

    await controller.handleWheel(event);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(controller.displayZoom).toBeGreaterThan(2);
    expect(controller.displayZoom).toBeLessThan(2.1);
  });

  it('zooms from a non-cancelable modified event without preventing default', async () => {
    const controller = createController();
    const event = wheelEvent({ ctrlKey: true, cancelable: false });

    await controller.handleWheel(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(controller.displayZoom).toBeGreaterThan(1);
  });

  it('admits modified wheel zoom in continuous layout', async () => {
    const controller = createController({ type: 'continuous', maxWidth: 600 });
    const event = wheelEvent({ ctrlKey: true });

    await controller.handleWheel(event);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(controller.displayZoom).toBeGreaterThan(1);
  });
});
