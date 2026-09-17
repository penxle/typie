import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { cdp } from 'vitest/browser';
import { PAGE_GAP } from './constants';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import { resolvePageSpans } from './geometry';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';
import type { CDPSession } from '@vitest/browser-playwright';
import type { EditorFrameSyncTestHarness } from './editor-frame-sync-test-host.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@mearie/svelte', async (importOriginal) => {
  const original = await importOriginal<typeof import('@mearie/svelte')>();
  return { ...original, createMutation: () => [vi.fn()] };
});

let mounted: Record<string, unknown> | undefined;
let editor: Editor;
afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  editor?.destroy();
  window.getSelection()?.removeAllRanges();
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({ node, children, modifiers: {} as never, carry: [] });
const paragraph = (text: string) => entry({ type: 'paragraph' }, [entry({ type: 'text', text })]);
const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

async function viewer(
  paginated: boolean,
  withZoom = false,
  useWindowScroll = false,
  { nativeSelection = true, mode = 'viewer' }: { nativeSelection?: boolean; mode?: 'editor' | 'viewer' } = {},
) {
  const plain: PlainDoc = {
    root: entry(
      {
        type: 'root',
        layout_mode: paginated
          ? {
              type: 'paginated',
              page_width: 320,
              page_height: 240,
              page_margin_top: 40,
              page_margin_bottom: 40,
              page_margin_left: 20,
              page_margin_right: 20,
            }
          : { type: 'continuous', max_width: 320 },
      },
      Array.from({ length: 120 }, () => paragraph('긴 한글 문장과 emoji 😀를 여러 줄에 걸쳐 선택합니다. '.repeat(6))),
    ),
  };
  editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  const ready = Promise.withResolvers<EditorFrameSyncTestHarness>();
  const target = document.createElement('div');
  document.body.append(target);
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: {
      editor,
      readOnly: true,
      nativeSelection,
      mode,
      withZoom,
      useWindowScroll,
      headerHeight: useWindowScroll ? 40 : 0,
      onReady: ready.resolve,
      userId: crypto.randomUUID(),
    },
  });
  const harness = await ready.promise;
  await expect.poll(() => editor.published?.frames.get(0)?.surface.isConnected).toBe(true);
  if (nativeSelection) await expect.poll(() => document.querySelector('[data-native-selection-layer].fonts-ready')).not.toBeNull();
  const root = nativeSelection ? document.querySelector<HTMLElement>('[data-native-selection-layer]') : editor.documentTrackEl;
  if (!root) throw new Error('Expected document body');
  return { root, ...harness };
}

it.for([false, true])(
  'preserves native text and selection while zooming, paginated=%s',
  { timeout: 30_000 },
  async (paginated, { task }) => {
    const { root } = await viewer(paginated);
    const spans = [...root.querySelectorAll<HTMLElement>('[data-selection-run]')];
    const first = spans[0].firstChild;
    const last = spans.at(-1)?.firstChild;
    const selection = window.getSelection();
    if (!first || !last || !selection) throw new Error('Expected selectable text');
    selection.setBaseAndExtent(first, 0, last, last.textContent?.length ?? 0);
    const text = selection.toString();
    const timings: number[] = [];
    for (const zoom of [1.137, 1.253, 1.371, 1.489, 1.607, 1.729]) {
      await frame();
      const started = performance.now();
      editor.displayZoom = zoom;
      await tick();
      await Promise.resolve();
      root.getBoundingClientRect();
      timings.push(performance.now() - started);
    }
    Object.assign(task.meta, { performance: { runs: spans.length, zoomUpdateMs: timings } });
    expect(spans.every((span) => span.isConnected)).toBe(true);
    expect(selection.anchorNode).toBe(first);
    expect(selection.focusNode).toBe(last);
    expect(selection.toString()).toBe(text);

    const zoom = editor.displayZoom;
    const pages = resolvePageSpans(editor.pageSizes, {
      displayZoom: zoom,
      scaleFactor: editor.scaleFactor,
      pageGap: paginated ? PAGE_GAP * zoom : 0,
    });
    const blocks = editor.published?.snapshot.selectionLayout;
    if (!blocks) throw new Error('Expected selection layout');
    const rootRect = root.getBoundingClientRect();
    for (const [index, span] of spans.entries()) {
      if (index % 97 !== 0 && index !== spans.length - 1) continue;
      const block = blocks[Number(span.closest<HTMLElement>('[data-selection-block]')?.dataset.selectionBlock)];
      const run = block.runs[Number(span.dataset.selectionRun)];
      const range = document.createRange();
      range.selectNodeContents(span);
      const rect = range.getBoundingClientRect();
      expect(Math.abs(rect.top - rootRect.top - pages[run.page_idx].top - run.rect.y * zoom)).toBeLessThan(0.25);
    }
  },
);

it.each([false, true])('presents the final raster scale after repeated document and browser zoom, paginated=%s', async (paginated) => {
  const { root } = await viewer(paginated, true);
  for (const deltaY of [-20, -20, -20, 20, 20, 20, -20]) {
    root.dispatchEvent(new WheelEvent('wheel', { deltaY, ctrlKey: true, bubbles: true, cancelable: true, clientX: 160, clientY: 80 }));
    await frame();
  }
  await expect.poll(() => editor.renderZoom).toBeCloseTo(editor.displayZoom, 5);
  const documentZoom = editor.displayZoom;
  for (const scale of [1, 1.25, 2, 1]) {
    vi.stubGlobal('devicePixelRatio', scale);
    editor.resizeViewport(360, 180, scale);
    await expect.poll(() => editor.published?.snapshot.viewport.scale_factor).toBe(scale);
    await expect
      .poll(() => {
        const surface = editor.published?.frames.get(0)?.surface;
        const canvas = surface?.querySelector<HTMLCanvasElement>(':scope [data-surface-layer="foreground"] canvas');
        return canvas?.isConnected ? new DOMMatrixReadOnly(canvas.style.transform).a : 0;
      })
      .toBeCloseTo(1 / (scale * documentZoom), 5);
    expect(editor.terminal).toBe(false);
    expect(editor.displayZoom).toBe(documentZoom);
  }
});

it.each([false, true])('leaves touch scrolling and pinch zoom to the browser, windowScroll=%s', async (useWindowScroll) => {
  const { root } = await viewer(false, true, useWindowScroll);
  const span = root.querySelector<HTMLElement>('[data-selection-run]');
  if (!span) throw new Error('Expected selection text');
  const rect = span.getBoundingClientRect();
  const contact = (identifier: number, x: number) => new Touch({ identifier, target: span, clientX: x, clientY: rect.top + 5 });
  const dispatch = (type: string, touches: Touch[]) => {
    const event = new TouchEvent(type, { touches, bubbles: true, cancelable: true });
    span.dispatchEvent(event);
    return event;
  };
  const first = contact(0, rect.left + 20);
  expect(dispatch('touchstart', [first]).defaultPrevented).toBe(false);
  expect(dispatch('touchmove', [first]).defaultPrevented).toBe(false);
  const second = contact(1, rect.left + 80);
  expect(dispatch('touchstart', [first, second]).defaultPrevented).toBe(false);
  const before = editor.displayZoom;
  expect(dispatch('touchmove', [first, contact(1, rect.left + 110)]).defaultPrevented).toBe(false);
  await tick();
  expect(editor.displayZoom).toBe(before);
  dispatch('touchend', []);
  if (useWindowScroll) {
    const outside = new TouchEvent('touchstart', {
      touches: [0, 1].map((identifier) => new Touch({ identifier, target: document.body, clientX: 100 + identifier * 60, clientY: 10 })),
      bubbles: true,
      cancelable: true,
    });
    document.body.dispatchEvent(outside);
    expect(outside.defaultPrevented).toBe(false);
  }
});

it.each(['ctrlKey', 'metaKey'] as const)('only claims %s wheel zoom over the document body', async (modifier) => {
  const { root } = await viewer(false, true, true);
  const span = root.querySelector<HTMLElement>('[data-selection-run]');
  const header = document.querySelector<HTMLElement>('[data-editor-test-header]');
  const controls = document.querySelector<HTMLElement>('[data-floating-editor-zoom-controls]');
  if (!span || !header || !controls) throw new Error('Expected viewer body, header and zoom controls');
  const wheel = (target: HTMLElement) => {
    const rect = target.getBoundingClientRect();
    const event = new WheelEvent('wheel', {
      bubbles: true,
      cancelable: true,
      [modifier]: true,
      deltaY: -40,
      clientX: rect.left + 5,
      clientY: rect.top + 5,
    });
    target.dispatchEvent(event);
    return event;
  };
  const before = editor.displayZoom;
  expect(wheel(header).defaultPrevented).toBe(false);
  expect(wheel(controls).defaultPrevented).toBe(false);
  await tick();
  expect(editor.displayZoom).toBe(before);
  expect(wheel(span).defaultPrevented).toBe(true);
  await expect.poll(() => editor.displayZoom).toBeGreaterThan(before);
});

it('keeps document zoom controls usable while leaving zoom shortcuts to the browser', async () => {
  await viewer(false, true);
  const zoomIn = document.querySelector<HTMLButtonElement>('[aria-label="페이지 확대"]');
  if (!zoomIn) throw new Error('Expected zoom control');
  zoomIn.focus();
  const before = editor.displayZoom;
  for (const modifier of ['ctrlKey', 'metaKey']) {
    for (const key of ['+', '-', '0']) {
      const event = new KeyboardEvent('keydown', { key, [modifier]: true, bubbles: true, cancelable: true });
      zoomIn.dispatchEvent(event);
      expect(event.defaultPrevented).toBe(false);
    }
  }
  await tick();
  expect(editor.displayZoom).toBe(before);
  zoomIn.click();
  await expect.poll(() => editor.displayZoom).toBeGreaterThan(before);
});

it('zooms the browser without changing document zoom with trusted touch input', async () => {
  const session: CDPSession = cdp();
  let touching = false;
  await session.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
  try {
    const { root } = await viewer(false, true);
    const rect = root.getBoundingClientRect();
    const offset = window.frameElement?.getBoundingClientRect();
    const x = rect.left + 140 + (offset?.left ?? 0);
    const y = rect.top + 50 + (offset?.top ?? 0);
    const before = await session.send('Page.getLayoutMetrics');
    const beforeZoom = editor.displayZoom;
    await session.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 0, x: x - 30, y }] });
    touching = true;
    await session.send('Input.dispatchTouchEvent', {
      type: 'touchStart',
      touchPoints: [
        { id: 0, x: x - 30, y },
        { id: 1, x: x + 30, y },
      ],
    });
    for (const distance of [40, 50, 60, 70]) {
      await session.send('Input.dispatchTouchEvent', {
        type: 'touchMove',
        touchPoints: [
          { id: 0, x: x - distance, y },
          { id: 1, x: x + distance, y },
        ],
      });
      await frame();
    }
    await session.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    touching = false;
    await expect
      .poll(async () => {
        const metrics = await session.send('Page.getLayoutMetrics');
        return metrics.cssVisualViewport.scale;
      })
      .toBeGreaterThan(before.cssVisualViewport.scale);
    expect(editor.displayZoom).toBe(beforeZoom);
  } finally {
    if (touching) await session.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
    await session.send('Emulation.setPageScaleFactor', { pageScaleFactor: 1 });
    await session.send('Emulation.clearDeviceMetricsOverride');
  }
});

it.each(['viewer', 'editor'] as const)('uses %s zoom policy for read-only content without native selection', async (mode) => {
  await viewer(false, true, true, { nativeSelection: false, mode });
  const input = editor.inputEl;
  const header = document.querySelector<HTMLElement>('[data-editor-test-header]');
  const page = editor.pageEls[0];
  if (!input || !header || !page) throw new Error('Expected editor input, header and page');
  const ownsZoom = mode === 'editor';
  const before = editor.displayZoom;
  const shortcut = new KeyboardEvent('keydown', { key: '+', ctrlKey: true, metaKey: true, bubbles: true, cancelable: true });
  input.dispatchEvent(shortcut);
  expect(shortcut.defaultPrevented).toBe(ownsZoom);
  if (ownsZoom) await expect.poll(() => editor.displayZoom).toBeGreaterThan(before);
  else expect(editor.displayZoom).toBe(before);

  const wheel = new WheelEvent('wheel', { deltaY: -40, ctrlKey: true, bubbles: true, cancelable: true });
  header.dispatchEvent(wheel);
  expect(wheel.defaultPrevented).toBe(ownsZoom);

  const rect = page.getBoundingClientRect();
  const contact = (identifier: number, distance: number) =>
    new Touch({ identifier, target: page, clientX: rect.left + 100 + distance, clientY: rect.top + 50 });
  const dispatch = (type: string, touches: Touch[]) => {
    const event = new TouchEvent(type, { touches, bubbles: true, cancelable: true });
    page.dispatchEvent(event);
    return event;
  };
  dispatch('touchstart', [contact(0, -30), contact(1, 30)]);
  expect(dispatch('touchmove', [contact(0, -50), contact(1, 50)]).defaultPrevented).toBe(ownsZoom);
  dispatch('touchcancel', []);
});
