import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  RENDER_ZOOM_MAX_COMMIT_DELAY_MS,
  RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS,
  RENDER_ZOOM_SCALE_RATIO_THRESHOLD,
} from '$lib/editor-ffi/zoom';
import { EditorZoomController } from './editor-zoom.svelte';
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

  it('commits continuous render zoom immediately at gesture end', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(0.8);
    controller.commitRenderZoom();

    expect(controller.renderZoom).toBe(0.8);
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

  it('defers a gesture-end commit until the minimum interval after an intermediate commit', () => {
    vi.useFakeTimers();
    const controller = createController({ type: 'continuous', maxWidth: 600 });

    controller.setZoom(1 / RENDER_ZOOM_SCALE_RATIO_THRESHOLD);
    const firstRenderZoom = controller.renderZoom;
    controller.setZoom(1.1);
    controller.commitRenderZoom();

    expect(controller.renderZoom).toBe(firstRenderZoom);

    vi.advanceTimersByTime(RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS);
    expect(controller.renderZoom).toBe(1.1);
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

  it('rebases tiny wheel zoom after the quiet timeout', async () => {
    vi.useFakeTimers();
    const controller = createController();

    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    vi.advanceTimersByTime(EditorZoomController.WHEEL_RAW_ZOOM_RESET_MS);
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3, timeStamp: EditorZoomController.WHEEL_RAW_ZOOM_RESET_MS }));

    expect(controller.displayZoom).toBe(1);
  });

  it('cancels the raw wheel zoom reset when zoom is set programmatically', async () => {
    vi.useFakeTimers();
    const controller = createController();
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    expect(vi.getTimerCount()).toBe(1);

    controller.setZoom(1, { commitRender: true });

    expect(vi.getTimerCount()).toBe(0);
  });

  it('cancels the raw wheel zoom reset when destroyed', async () => {
    vi.useFakeTimers();
    const controller = createController();
    await controller.handleWheel(wheelEvent({ metaKey: true, deltaY: -3 }));
    expect(vi.getTimerCount()).toBe(1);

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
    expect(controller.displayZoom).toBe(2);
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
