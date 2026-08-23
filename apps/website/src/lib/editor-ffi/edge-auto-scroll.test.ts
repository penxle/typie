import { afterEach, describe, expect, it, vi } from 'vitest';
import { EditorEdgeAutoScroll } from './edge-auto-scroll';
import type { ScrollViewport } from '@typie/ui/utils';
import type { Editor } from './editor.svelte';

const VIEWPORT_RECT = { top: 0, bottom: 200, left: 0, right: 200 };

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

const setup = () => {
  let scrollTop = 0;
  const events: { type: 'notify' | 'onScroll'; scrollTop: number }[] = [];

  const viewport: ScrollViewport = {
    target: document.createElement('div'),
    getRect: () => VIEWPORT_RECT,
    getScrollTop: () => scrollTop,
    getScrollLeft: () => 0,
    getScrollHeight: () => 10_000,
    scrollBy: (_x, y) => {
      scrollTop += y;
    },
    scrollTo: (options) => {
      if (options.top !== undefined) scrollTop = options.top;
    },
  };

  const editor = {
    scrollViewport: viewport,
    notifyViewportScrolled: () => {
      events.push({ type: 'notify', scrollTop });
    },
  } as unknown as Editor;

  return {
    editor,
    events,
    getScrollTop: () => scrollTop,
    onScroll: () => {
      events.push({ type: 'onScroll', scrollTop });
    },
  };
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('EditorEdgeAutoScroll', () => {
  it('reports the moved viewport to the editor before handing the step to its owner', () => {
    const { editor, events, getScrollTop, onScroll } = setup();
    const frames = installAnimationFrames();
    const autoScroll = new EditorEdgeAutoScroll();

    autoScroll.update(editor, { clientX: 100, clientY: 190 }, onScroll);
    frames.runNext(16);
    frames.runNext(32);

    expect(getScrollTop()).toBeGreaterThan(0);
    expect(events).toEqual([
      { type: 'notify', scrollTop: getScrollTop() },
      { type: 'onScroll', scrollTop: getScrollTop() },
    ]);

    autoScroll.stop();
  });

  it('reports every scrolled frame so a stale viewport anchor cannot pin the scroll', () => {
    const { editor, events, onScroll } = setup();
    const frames = installAnimationFrames();
    const autoScroll = new EditorEdgeAutoScroll();

    autoScroll.update(editor, { clientX: 100, clientY: 190 }, onScroll);
    frames.runNext(16);
    for (let index = 1; index <= 5; index += 1) {
      frames.runNext(16 + index * 16);
    }

    const notified = events.filter((event) => event.type === 'notify').map((event) => event.scrollTop);
    expect(notified).toHaveLength(5);
    expect(notified.every((scrollTop, index) => index === 0 || scrollTop > notified[index - 1])).toBe(true);

    autoScroll.stop();
  });
});
