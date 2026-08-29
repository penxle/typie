import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { CURSOR_VISIBLE_MARGIN, PAGE_GAP } from './constants';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import { pageRectsToClientRect, pageRectToClientRect, selectionHeadRect } from './geometry';
import { computeSelectionHandleVisual } from './gesture.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';
import type { EditorFrameSyncTestHarness } from './editor-frame-sync-test-host.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const PAGE_WIDTH = 320;
const PAGE_HEIGHT = 220;
const PAGE_MARGIN = 24;
const REPEAT_INTERVAL_MS = 33;
const REPEAT_START_PHASES_MS = [0, 16] as const;
const REPEAT_EVENTS_PER_LEG = 16;
const REPEAT_CYCLES_PER_PHASE = 3;
const EDIT_REPEAT_EVENTS_PER_LEG = 60;
const MAXIMUM_DISPLAY_FRAME_GAP_MS = 250;
const MAXIMUM_PUBLICATION_STALL_MS = 250;
const COORDINATE_TOLERANCE_PX = 1;
const PERFORMANCE_PARAGRAPH_COUNT = 300;
const PERFORMANCE_PARAGRAPH_TEXT = '0123456789'.repeat(10);

class SilentIntersectionObserver implements IntersectionObserver {
  readonly root = null;
  readonly rootMargin = '0px';
  readonly scrollMargin = '0px';
  readonly thresholds = [0];

  disconnect(): void {
    return;
  }

  observe(): void {
    return;
  }

  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }

  unobserve(): void {
    return;
  }
}

type RepeatKey = 'ArrowUp' | 'ArrowDown' | 'Enter' | 'Backspace';

type WebFrameSample = {
  phaseMs: number;
  frame: number;
  capturedAtMs: number;
  direction: RepeatKey | null;
  revision: number;
  cursorPage: number;
  hasNativeFrame: boolean;
  scrollTop: number;
  documentHeight: number;
  cursorDocumentY: number;
  highlightDocumentY: number;
  expectedCursorY: number;
  actualCursorY: number | null;
  expectedHighlightY: number;
  actualHighlightY: number | null;
  caretOnCursorPage: boolean;
  highlightOnExpectedContainer: boolean;
  selectionHeadVisible: boolean;
};

function matchingWebFrameSample(overrides: Partial<WebFrameSample> = {}): WebFrameSample {
  return {
    phaseMs: 0,
    frame: 0,
    capturedAtMs: 0,
    direction: 'ArrowDown',
    revision: 1,
    cursorPage: 0,
    hasNativeFrame: true,
    scrollTop: 0,
    documentHeight: 100,
    cursorDocumentY: 10,
    highlightDocumentY: 8,
    expectedCursorY: 10,
    actualCursorY: 10,
    expectedHighlightY: 8,
    actualHighlightY: 8,
    caretOnCursorPage: true,
    highlightOnExpectedContainer: true,
    selectionHeadVisible: true,
    ...overrides,
  };
}

let mounted: Record<string, unknown> | undefined;
let editor: Editor | undefined;

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  editor?.destroy();
  editor = undefined;
  document.body.replaceChildren();
});

function entry(node: PlainNode, children: PlainNodeEntry[] = [], modifiers: PlainNodeEntry['modifiers'] = {} as never): PlainNodeEntry {
  return { node, modifiers, carry: [], children };
}

function doc(text = '', href?: string): PlainDoc {
  const textChildren = text ? [entry({ type: 'text', text }, [], href ? ({ link: { type: 'link', href } } as never) : ({} as never))] : [];
  return {
    root: entry(
      {
        type: 'root',
        layout_mode: {
          type: 'paginated',
          page_width: PAGE_WIDTH,
          page_height: PAGE_HEIGHT,
          page_margin_top: PAGE_MARGIN,
          page_margin_bottom: PAGE_MARGIN,
          page_margin_left: PAGE_MARGIN,
          page_margin_right: PAGE_MARGIN,
        },
      },
      [entry({ type: 'paragraph' }, textChildren)],
      {
        block_gap: { type: 'block_gap', value: 120 },
        font_size: { type: 'font_size', value: 1600 },
        line_height: { type: 'line_height', value: 160 },
      } as never,
    ),
  };
}

function continuousDoc(text = ''): PlainDoc {
  const document = doc(text);
  return {
    ...document,
    root: {
      ...document.root,
      node: { type: 'root', layout_mode: { type: 'continuous', max_width: 600 } },
    },
  };
}

function longDoc(): PlainDoc {
  const document = doc();
  return {
    ...document,
    root: {
      ...document.root,
      node: {
        type: 'root',
        layout_mode: { type: 'continuous', max_width: PAGE_WIDTH },
      },
      children: Array.from({ length: PERFORMANCE_PARAGRAPH_COUNT }, () =>
        entry({ type: 'paragraph' }, [entry({ type: 'text', text: PERFORMANCE_PARAGRAPH_TEXT })]),
      ),
      modifiers: {
        block_gap: { type: 'block_gap', value: 80 },
        font_size: { type: 'font_size', value: 1000 },
        line_height: { type: 'line_height', value: 150 },
      } as never,
    },
  };
}

function foldedTrackedRangeDoc(targetText: string): PlainDoc {
  return {
    root: entry(
      { type: 'root', layout_mode: { type: 'continuous', max_width: PAGE_WIDTH } },
      [
        ...Array.from({ length: 12 }, (_, index) =>
          entry({ type: 'paragraph' }, [entry({ type: 'text', text: `preceding paragraph ${index}` })]),
        ),
        entry({ type: 'fold' }, [
          entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'Collapsed section' })]),
          entry({ type: 'fold_content' }, [entry({ type: 'paragraph' }, [entry({ type: 'text', text: targetText })])]),
        ]),
        entry({ type: 'paragraph' }),
      ],
      {
        block_gap: { type: 'block_gap', value: 80 },
        font_size: { type: 'font_size', value: 1000 },
        line_height: { type: 'line_height', value: 150 },
      } as never,
    ),
  };
}

function nestedFoldedSelectionDoc(targetText: string): PlainDoc {
  return {
    root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: PAGE_WIDTH } }, [
      entry({ type: 'fold' }, [
        entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'Outer fold' })]),
        entry({ type: 'fold_content' }, [
          entry({ type: 'fold' }, [
            entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'Inner fold' })]),
            entry({ type: 'fold_content' }, [entry({ type: 'paragraph' }, [entry({ type: 'text', text: targetText })])]),
          ]),
        ]),
      ]),
    ]),
  };
}

function paginatedDocWithPageBreaks(pageCount: number, linkedLastPage?: { text: string; href: string }): PlainDoc {
  const pageHeight = 221;
  return {
    root: entry(
      {
        type: 'root',
        layout_mode: {
          type: 'paginated',
          page_width: PAGE_WIDTH,
          page_height: pageHeight,
          page_margin_top: PAGE_MARGIN,
          page_margin_bottom: PAGE_MARGIN,
          page_margin_left: PAGE_MARGIN,
          page_margin_right: PAGE_MARGIN,
        },
      },
      Array.from({ length: pageCount }, (_, page) => {
        if (page < pageCount - 1) return entry({ type: 'paragraph' }, [entry({ type: 'page_break' })]);
        if (!linkedLastPage) return entry({ type: 'paragraph' });
        return entry({ type: 'paragraph' }, [
          entry({ type: 'text', text: linkedLastPage.text }, [], { link: { type: 'link', href: linkedLastPage.href } } as never),
        ]);
      }),
    ),
  };
}

function paginatedDocWithLinkedPage(pageCount: number, linkedPage: number, text: string, href: string): PlainDoc {
  const pageHeight = 221;
  return {
    root: entry(
      {
        type: 'root',
        layout_mode: {
          type: 'paginated',
          page_width: PAGE_WIDTH,
          page_height: pageHeight,
          page_margin_top: PAGE_MARGIN,
          page_margin_bottom: PAGE_MARGIN,
          page_margin_left: PAGE_MARGIN,
          page_margin_right: PAGE_MARGIN,
        },
      },
      Array.from({ length: pageCount }, (_, page) => {
        const children: PlainNodeEntry[] = [];
        if (page === linkedPage) {
          children.push(entry({ type: 'text', text }, [], { link: { type: 'link', href } } as never));
        }
        if (page < pageCount - 1) children.push(entry({ type: 'page_break' }));
        return entry({ type: 'paragraph' }, children);
      }),
    ),
  };
}

async function mountEditor(
  plain: PlainDoc,
  options: {
    readOnly?: boolean;
    typewriterEnabled?: boolean;
    withZoom?: boolean;
    displayZoom?: number;
    contentInsetLeft?: number;
    contentInsetRight?: number;
  } = {},
) {
  editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  if (options.displayZoom !== undefined) {
    editor.displayZoom = options.displayZoom;
    editor.commitRenderZoom(options.displayZoom);
  }
  const target = document.createElement('div');
  document.body.append(target);
  const harness = Promise.withResolvers<EditorFrameSyncTestHarness>();
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: {
      editor,
      onReady: harness.resolve,
      readOnly: options.readOnly,
      typewriterEnabled: options.typewriterEnabled,
      userId: `frame-sync-${crypto.randomUUID()}`,
      withZoom: options.withZoom,
      contentInsetLeft: options.contentInsetLeft,
      contentInsetRight: options.contentInsetRight,
    },
  });
  const result = await harness.promise;
  await tick();
  await vi.waitFor(() => expect(editor?.published?.frames.get(0)).toBeDefined());
  return { editor, ...result };
}

async function mountEditorWithPublishedReady(plain: PlainDoc, options: { headerHeight?: number } = {}) {
  editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  const target = document.createElement('div');
  document.body.append(target);
  const mountedReady = Promise.withResolvers<EditorFrameSyncTestHarness>();
  let publishedReady = false;
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: {
      editor,
      onReady: mountedReady.resolve,
      onPublishedReady: () => {
        publishedReady = true;
      },
      userId: `frame-sync-ready-${crypto.randomUUID()}`,
      headerHeight: options.headerHeight,
    },
  });
  const harness = await mountedReady.promise;
  return { editor, publishedReady: () => publishedReady, ...harness };
}

function dispatchEditorKey(editor: Editor, key: RepeatKey, init: KeyboardEventInit = {}): number {
  const input = editor.inputEl;
  if (!input) throw new Error('Production editor input is not mounted');
  const beforeRevision = editor.appliedRevision;
  input.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
  return beforeRevision;
}

function editorTouch(target: EventTarget, identifier: number, clientX: number, clientY: number): Touch {
  return new Touch({ identifier, target, clientX, clientY });
}

function dispatchEditorTouch(target: HTMLElement, type: string, touches: Touch[]): TouchEvent {
  const event = new TouchEvent(type, { touches, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

async function pressEditorKey(editor: Editor, key: 'Enter' | 'Backspace') {
  const beforeRevision = dispatchEditorKey(editor, key);
  await expect.poll(() => editor.appliedRevision).toBeGreaterThan(beforeRevision);
  await tick();
}

function expectSelectionHeadVisible(editor: Editor, scrollRoot: HTMLElement, context: string): void {
  const target = selectionHeadRect(editor.published?.snapshot);
  const clientRect = target && pageRectToClientRect(editor, target);
  const viewport = scrollRoot.getBoundingClientRect();
  expect(clientRect, `${context}: the published selection head has no mounted client geometry`).not.toBeNull();
  expect(
    clientRect &&
      clientRect.top >= viewport.top - COORDINATE_TOLERANCE_PX &&
      clientRect.bottom <= viewport.bottom + COORDINATE_TOLERANCE_PX,
    `${context}: selection head=${clientRect ? `${clientRect.top}..${clientRect.bottom}` : 'missing'} viewport=${viewport.top}..${viewport.bottom} scrollTop=${scrollRoot.scrollTop} scrollHeight=${scrollRoot.scrollHeight}`,
  ).toBe(true);
}

function readWebFrameSample(
  editor: Editor,
  scrollRoot: HTMLElement,
  phaseMs: number,
  frame: number,
  direction: RepeatKey | null,
): WebFrameSample {
  const published = editor.published;
  const cursor = published?.snapshot.cursor;
  if (!published || !cursor) throw new Error('Expected a published cursor');

  const paginated = published.snapshot.rootAttrs?.layout_mode.type === 'paginated';
  const zoom = editor.safeDisplayZoom();
  const pageGap = paginated ? PAGE_GAP * zoom : 0;
  const pageTop = published.snapshot.pageSizes
    .slice(0, cursor.page_idx)
    .reduce((sum, pageSize) => sum + pageSize.height * zoom + pageGap, 0);
  const rootTop = scrollRoot.getBoundingClientRect().top;
  const scrollTop = scrollRoot.scrollTop;
  const cursorDocumentY = pageTop + cursor.caret.y * zoom;
  const highlightDocumentY = pageTop + cursor.line.y * zoom;
  const caret = document.querySelector<HTMLElement>('[data-editor-caret]');
  const lineHighlight = document.querySelector<HTMLElement>('[data-editor-line-highlight]');
  const cursorPage = editor.pageEls[cursor.page_idx];
  const expectedHighlightContainer = paginated ? cursorPage : editor.extensionAreaEl;
  const cursorTop = caret?.getBoundingClientRect().top ?? null;
  const cursorBottom = cursorTop === null ? null : cursorTop + cursor.caret.height * zoom;
  const viewportBottom = rootTop + scrollRoot.clientHeight;

  return {
    phaseMs,
    frame,
    capturedAtMs: performance.now(),
    direction,
    revision: published.snapshot.revision,
    cursorPage: cursor.page_idx,
    hasNativeFrame: published.frames.has(cursor.page_idx),
    scrollTop,
    documentHeight: published.snapshot.pageSizes.reduce((sum, size) => sum + size.height, 0),
    cursorDocumentY,
    highlightDocumentY,
    expectedCursorY: rootTop + cursorDocumentY - scrollTop,
    actualCursorY: cursorTop,
    expectedHighlightY: rootTop + highlightDocumentY - scrollTop,
    actualHighlightY: lineHighlight?.getBoundingClientRect().top ?? null,
    caretOnCursorPage: caret?.parentElement === cursorPage,
    highlightOnExpectedContainer: lineHighlight?.parentElement === expectedHighlightContainer,
    selectionHeadVisible:
      cursorTop !== null &&
      cursorBottom !== null &&
      cursorTop >= rootTop - COORDINATE_TOLERANCE_PX &&
      cursorBottom <= viewportBottom + COORDINATE_TOLERANCE_PX,
  };
}

function nextAnimationFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

function delayUntil(deadline: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, Math.max(0, deadline - performance.now()));
  });
}

function describeWebFrameMismatch(sample: WebFrameSample, previous?: WebFrameSample): string | undefined {
  const nativePresentationMismatch = !sample.hasNativeFrame || !sample.caretOnCursorPage || !sample.highlightOnExpectedContainer;
  const cursorMismatch = sample.actualCursorY === null || Math.abs(sample.actualCursorY - sample.expectedCursorY) > COORDINATE_TOLERANCE_PX;
  const highlightMismatch =
    sample.actualHighlightY === null || Math.abs(sample.actualHighlightY - sample.expectedHighlightY) > COORDINATE_TOLERANCE_PX;
  if (!nativePresentationMismatch && !cursorMismatch && !highlightMismatch && sample.selectionHeadVisible) return;

  const cursorDocumentDelta = previous ? sample.cursorDocumentY - previous.cursorDocumentY : null;
  const highlightDocumentDelta = previous ? sample.highlightDocumentY - previous.highlightDocumentY : null;
  const scrollDelta = previous ? sample.scrollTop - previous.scrollTop : null;
  const cursorViewportDelta =
    previous?.actualCursorY !== null && previous?.actualCursorY !== undefined && sample.actualCursorY !== null
      ? sample.actualCursorY - previous.actualCursorY
      : null;
  const highlightViewportDelta =
    previous?.actualHighlightY !== null && previous?.actualHighlightY !== undefined && sample.actualHighlightY !== null
      ? sample.actualHighlightY - previous.actualHighlightY
      : null;

  return [
    `phase=${sample.phaseMs}ms frame=${sample.frame} direction=${sample.direction} revision=${sample.revision}`,
    `scrollTop=${sample.scrollTop} cursorPage=${sample.cursorPage} hasNativeFrame=${sample.hasNativeFrame}`,
    `caretOnCursorPage=${sample.caretOnCursorPage} highlightOnExpectedContainer=${sample.highlightOnExpectedContainer}`,
    `selectionHeadVisible=${sample.selectionHeadVisible}`,
    `cursor expected=${sample.expectedCursorY} actual=${sample.actualCursorY}`,
    `highlight expected=${sample.expectedHighlightY} actual=${sample.actualHighlightY}`,
    `delta cursorDocument=${cursorDocumentDelta} highlightDocument=${highlightDocumentDelta} scroll=${scrollDelta} ` +
      `cursorViewport=${cursorViewportDelta} highlightViewport=${highlightViewportDelta}`,
  ].join('\n');
}

function repeatKeys(first: RepeatKey, second: RepeatKey, eventsPerLeg: number, cycles: number): RepeatKey[] {
  return Array.from({ length: cycles }).flatMap(() => [
    ...Array.from({ length: eventsPerLeg }, () => first),
    ...Array.from({ length: eventsPerLeg }, () => second),
  ]);
}

async function driveWebRepeatPhase(
  editor: Editor,
  scrollRoot: HTMLElement,
  phaseMs: number,
  keys: RepeatKey[],
  scenario: string,
): Promise<WebFrameSample[]> {
  await nextAnimationFrame();
  const startedAt = performance.now();
  const samples: WebFrameSample[] = [];
  let currentKey: RepeatKey | null = null;
  let done = false;
  let cancelled = false;
  let inputError: unknown;

  const input = (async () => {
    try {
      for (const [index, key] of keys.entries()) {
        await delayUntil(startedAt + phaseMs + index * REPEAT_INTERVAL_MS);
        if (cancelled) break;
        currentKey = key;
        dispatchEditorKey(editor, key);
      }
    } catch (err) {
      inputError = err;
    } finally {
      done = true;
    }
  })();

  let settleFrames = 0;
  while (!done || settleFrames < 1) {
    await nextAnimationFrame();
    const sample = readWebFrameSample(editor, scrollRoot, phaseMs, samples.length, done ? null : currentKey);
    samples.push(sample);
    const mismatch = describeWebFrameMismatch(sample, samples.at(-2));
    if (mismatch) {
      cancelled = true;
      const screenshot = await page.screenshot({ element: scrollRoot, save: true }).catch((err: unknown) => `unavailable (${String(err)})`);
      await input;
      throw new Error(`Web ${scenario} presentation mismatch; screenshot=${screenshot}\n${mismatch}`);
    }
    if (done) settleFrames += 1;
  }

  await input;
  if (inputError) throw inputError;
  return samples;
}

function expectWebFrameLiveness(samples: WebFrameSample[], phaseMs: number) {
  let lastPublishedRevision = samples[0]?.revision;
  let lastPublicationAtMs = samples[0]?.capturedAtMs ?? 0;

  for (let index = 1; index < samples.length; index += 1) {
    const previous = samples[index - 1];
    const current = samples[index];
    const displayFrameGapMs = current.capturedAtMs - previous.capturedAtMs;
    expect(
      previous.direction === null || displayFrameGapMs <= MAXIMUM_DISPLAY_FRAME_GAP_MS,
      `Repeated input dropped display frames at phase ${phaseMs}ms: ` +
        `frames=${previous.frame}→${current.frame} gap=${displayFrameGapMs}ms revision=${current.revision}`,
    ).toBe(true);

    if (current.revision !== lastPublishedRevision) {
      lastPublishedRevision = current.revision;
      lastPublicationAtMs = current.capturedAtMs;
      continue;
    }
    const publicationStallMs = current.capturedAtMs - lastPublicationAtMs;
    expect(
      current.direction === null || publicationStallMs <= MAXIMUM_PUBLICATION_STALL_MS,
      `Repeated input stalled publication at phase ${phaseMs}ms: ` +
        `revision=${current.revision} stall=${publicationStallMs}ms frame=${current.frame}`,
    ).toBe(true);
  }
}

async function resetWebLongDocumentStart(editor: Editor, scrollRoot: HTMLElement) {
  editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
  scrollRoot.scrollTop = 0;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await waitForPresentation(editor);
}

async function waitForPresentation(editor: Editor, revision = editor.appliedRevision) {
  await expect.poll(() => editor.isPublished(revision, { requireFrame: true })).toBe(true);
  await tick();
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function prepareTrackedItemSmoothReveal(pageCount: number, targetPage: number) {
  const errorId = `smooth-target-${crypto.randomUUID()}`;
  const text = 'tracked target';
  const href = `https://example.com/${errorId}`;
  const harness = await mountEditor(paginatedDocWithLinkedPage(pageCount, targetPage, text, href));
  const { editor, scrollRoot } = harness;

  const targetUpdate = editor.updateNow((request) => {
    request.enqueue({ type: 'selection', op: { type: 'set_at', page: targetPage, x: PAGE_MARGIN, y: PAGE_MARGIN } });
  });
  expect(targetUpdate).not.toBeNull();
  if (!targetUpdate) throw new Error('Expected the tracked-target selection update');
  await waitForPresentation(editor, targetUpdate.revision);

  const selection = editor.appliedSnapshot.selection;
  const targetSelection = selection && editor.modifierSpanSelection(selection.head, 'link');
  expect(targetSelection).toBeDefined();
  if (!targetSelection) throw new Error('Expected the linked target selection');
  editor.setSpellcheckErrors([{ id: errorId, selection: targetSelection, context: text, corrections: [], explanation: '' }]);
  await expect.poll(() => editor.appliedSnapshot.trackedRanges.some((range) => range.id === errorId)).toBe(true);
  await waitForPresentation(editor);

  let startPresentation: Promise<void> | undefined;
  const startUpdate = editor.updateNow((request) => {
    request.enqueue({ type: 'selection', op: { type: 'set_at', page: pageCount - 1, x: PAGE_MARGIN, y: PAGE_MARGIN } });
    startPresentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard', behavior: 'instant' });
  });
  expect(startUpdate).not.toBeNull();
  expect(startPresentation).toBeDefined();
  if (!startUpdate || !startPresentation) throw new Error('Expected the smooth-reveal start-position presentation');
  await startPresentation;
  expect(scrollRoot.scrollTop).toBeGreaterThan(0);

  const unrelatedSelectionUpdate = editor.updateNow((request) => {
    request.enqueue({ type: 'selection', op: { type: 'set_at', page: targetPage + 10, x: PAGE_MARGIN, y: PAGE_MARGIN } });
  });
  expect(unrelatedSelectionUpdate).not.toBeNull();
  if (!unrelatedSelectionUpdate) throw new Error('Expected the unrelated selection update');
  await waitForPresentation(editor, unrelatedSelectionUpdate.revision);

  const presentation = editor.scrollIntoView({ target: { type: 'tracked_item', id: errorId }, policy: 'reveal', behavior: 'smooth' });
  expect(presentation).toBeDefined();
  if (!presentation) throw new Error('Expected the smooth reveal presentation');
  return { editor, errorId, presentation, scrollRoot };
}

function insertPagesBeforeTrackedTarget(editor: Editor, count = 1): number {
  const selection = editor.appliedSnapshot.selection;
  const savedSelection = selection && editor.freezeSelection(selection);
  expect(savedSelection).toBeDefined();
  if (!savedSelection) throw new Error('Expected a stable selection before the preceding page insertion');
  const update = editor.updateNow((request) => {
    request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } });
    for (let index = 0; index < count; index += 1) {
      request.enqueue({ type: 'insertion', op: { type: 'break', kind: 'page' } });
    }
    request.enqueue({ type: 'selection', op: { type: 'set_frozen', selection: savedSelection } });
  });
  expect(update).not.toBeNull();
  if (!update) throw new Error('Expected the preceding page insertion');
  return update.revision;
}

function trackedTargetScrollTop(editor: Editor, id: string): number | null {
  const snapshot = editor.published?.snapshot;
  const rect = snapshot && editor.trackedRangeForSnapshot(id, snapshot)?.rects[0];
  if (!snapshot || !rect) return null;
  const pageTop = snapshot.pageSizes.slice(0, rect.page_idx).reduce((sum, page) => sum + page.height + PAGE_GAP, 0);
  return pageTop + rect.rect.y - CURSOR_VISIBLE_MARGIN;
}

function expectActualCanvas(editor: Editor, pageIndex: number, requirePaintedPixels = true) {
  const frame = editor.published?.frames.get(pageIndex);
  const canvas = document.querySelector<HTMLCanvasElement>(`canvas[data-page-canvas="${pageIndex}"]`);
  expect(frame, `page ${pageIndex} must have a frame in the published bundle`).toBeDefined();
  expect(canvas, `page ${pageIndex} must have a production Page canvas`).toBe(frame?.canvas);
  if (requirePaintedPixels) {
    const pixels = canvas?.getContext('2d')?.getImageData(0, 0, canvas.width, canvas.height).data;
    expect(pixels && pixels.some((channel) => channel !== 0), `page ${pageIndex} canvas must contain rendered pixels`).toBe(true);
  }
}

describe('web editor frame synchronization', () => {
  it('resolves pointer coordinates from the document track without measuring individual pages', async () => {
    const { editor } = await mountEditor(paginatedDocWithPageBreaks(3));
    await waitForPresentation(editor);
    await tick();
    const firstPage = editor.pageEls[0];
    expect(firstPage).toBeDefined();
    expect(editor.documentTrackEl).toBe(document.querySelector('[data-editor-document-track]'));
    if (!firstPage) throw new Error('Expected the first page element');
    const firstPageRect = firstPage.getBoundingClientRect();
    const pageMeasurements = Object.values(editor.pageEls).flatMap((page) => (page ? [vi.spyOn(page, 'getBoundingClientRect')] : []));
    for (const measurePage of pageMeasurements) measurePage.mockClear();

    const local = editor.clientToLocal(firstPageRect.left + 10, firstPageRect.top + 20);

    expect(local).toMatchObject({ page: 0, x: 10, y: 20 });
    for (const measurePage of pageMeasurements) expect(measurePage).not.toHaveBeenCalled();
  });

  it('reveals search matches instantly while preserving smooth tracked-item reveals by default', async () => {
    const { editor } = await mountEditor(doc('match between match'));
    const reveals: Parameters<Editor['scrollIntoView']>[0][] = [];
    editor.registerScrollIntoView((options) => {
      reveals.push(options);
      return Promise.resolve();
    });

    editor.search('match');
    await expect.poll(() => reveals.length).toBe(1);

    editor.findNext();
    await expect.poll(() => reveals.length).toBe(2);

    editor.findPrevious();
    await expect.poll(() => reveals.length).toBe(3);

    editor.replace('replacement');
    await expect.poll(() => editor.searchMatches.length).toBe(1);
    await expect.poll(() => reveals.length).toBe(4);

    editor.revealTrackedItem('search-match:1');
    await expect.poll(() => reveals.length).toBe(5);

    expect(reveals).toEqual([
      { target: { type: 'tracked_item', id: 'search-match:0' }, policy: 'reveal', behavior: 'instant' },
      { target: { type: 'tracked_item', id: 'search-match:1' }, policy: 'reveal', behavior: 'instant' },
      { target: { type: 'tracked_item', id: 'search-match:0' }, policy: 'reveal', behavior: 'instant' },
      { target: { type: 'tracked_item', id: 'search-match:1' }, policy: 'reveal', behavior: 'instant' },
      { target: { type: 'tracked_item', id: 'search-match:1' }, policy: 'reveal', behavior: 'smooth' },
    ]);
  });

  it.each([
    { mode: 'editable', readOnly: false },
    { mode: 'read-only', readOnly: true },
  ])('expands a collapsed fold before revealing a tracked item in $mode mode', async ({ readOnly }) => {
    const errorId = `folded-error-${crypto.randomUUID()}`;
    const targetText = 'folded tracked target';
    const { editor, scrollRoot } = await mountEditor(foldedTrackedRangeDoc(targetText), { readOnly });
    const prose = editor.proseText();
    const start = prose.indexOf(targetText);
    expect(start).toBeGreaterThanOrEqual(0);
    const selection = editor.proseToSelection(start, start + targetText.length);
    expect(selection).toBeDefined();
    if (!selection) throw new Error('Expected the folded prose target to resolve');

    editor.setSpellcheckErrors([{ id: errorId, selection, context: targetText, corrections: [], explanation: '' }]);
    await expect.poll(() => editor.appliedSnapshot.trackedRanges.some((range) => range.id === errorId)).toBe(true);
    await waitForPresentation(editor);
    expect(editor.trackedRangeForSnapshot(errorId, editor.appliedSnapshot)?.rects).toEqual([]);

    editor.setActiveSpellcheckError(errorId);

    await expect.poll(() => editor.trackedRangeForSnapshot(errorId, editor.appliedSnapshot)?.rects.length ?? 0).toBeGreaterThan(0);
    await waitForPresentation(editor);
    await vi.waitFor(() => {
      const published = editor.published;
      const range = published && editor.trackedRangeForSnapshot(errorId, published.snapshot);
      const targetRect = range && pageRectsToClientRect(editor, range.rects);
      const viewport = scrollRoot.getBoundingClientRect();
      expect(targetRect).not.toBeNull();
      expect(targetRect?.top).toBeGreaterThanOrEqual(viewport.top - COORDINATE_TOLERANCE_PX);
      expect(targetRect?.bottom).toBeLessThanOrEqual(viewport.bottom + COORDINATE_TOLERANCE_PX);
    });
  });

  it('presents ordinary page overlays without waiting for IntersectionObserver delivery', async () => {
    vi.stubGlobal('IntersectionObserver', SilentIntersectionObserver);
    try {
      const href = 'https://example.com/first-frame-overlay';
      const { editor } = await mountEditor(paginatedDocWithPageBreaks(3, { text: 'linked pixels', href }));
      const update = editor.updateNow((request) => {
        request.enqueue({ type: 'selection', op: { type: 'set_at', page: 2, x: PAGE_MARGIN, y: PAGE_MARGIN } });
        editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
      });
      expect(update).not.toBeNull();
      if (!update) throw new Error('Expected the far-page reveal update');

      let presented = false;
      for (let frame = 0; frame < 60; frame += 1) {
        await nextAnimationFrame();
        if ((editor.published?.snapshot.revision ?? -1) < update.revision) continue;
        expectActualCanvas(editor, 2);
        expect(editor.pageEls[2]?.querySelector(`a[aria-label="${href}"]`)).not.toBeNull();
        presented = true;
        break;
      }
      expect(presented, 'the far-page reveal was not presented within 60 frames').toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('notifies the real ready gate after the first accepted frame', async () => {
    const mountedEditor = await mountEditorWithPublishedReady(doc());

    await expect.poll(() => mountedEditor.editor.isPublished(mountedEditor.editor.appliedRevision, { requireFrame: true })).toBe(true);

    expect(mountedEditor.publishedReady()).toBe(true);
  });

  it('prepares the nearest page for the first frame when the initial viewport does not reach the first page', async () => {
    // 뷰포트 높이 180, 헤더 400: 확장 뷰포트 [-180, 360]이 페이지 시작(400)에 닿지 않는다.
    // 최근접 페이지 폴백이 없으면 프레임 0개 publication이 수용되어 ready 게이트가 영구 교착한다.
    const mountedEditor = await mountEditorWithPublishedReady(doc('hello'), { headerHeight: 400 });

    await expect.poll(() => mountedEditor.publishedReady()).toBe(true);
    expect(mountedEditor.editor.published?.frames.has(0)).toBe(true);
  });

  it('fires the ready gate when the initial viewport reaches the first page below a header', async () => {
    // 헤더 100: 확장 뷰포트 [-180, 360]이 페이지 시작(100)과 교차한다.
    const mountedEditor = await mountEditorWithPublishedReady(doc('hello'), { headerHeight: 100 });

    await expect.poll(() => mountedEditor.publishedReady()).toBe(true);
  });

  it('completes a pending reveal without removing the last frame after terminal surface failure', async () => {
    const { editor } = await mountEditor(doc());
    const canvas = document.querySelector<HTMLCanvasElement>('canvas[data-page-canvas="0"]');
    expect(canvas).not.toBeNull();
    let presentation: Promise<void> | undefined;

    editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } });
      presentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
    editor.surfaceReplacementFailed(0);

    await expect(presentation).resolves.toBeUndefined();
    await tick();
    expect(editor.terminal).toBe(true);
    expect(canvas?.isConnected).toBe(true);
  });

  it('presents a restored selection at its revealed scroll position', async () => {
    const { editor, scrollRoot } = await mountEditor(longDoc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: 1_000_000 } }));
    await waitForPresentation(editor);
    const selection = editor.appliedSnapshot.selection;
    const saved = selection && editor.freezeSelection(selection);
    expect(saved).toBeDefined();
    if (!saved) throw new Error('Expected a restorable selection');

    await resetWebLongDocumentStart(editor, scrollRoot);
    expect(scrollRoot.scrollTop).toBe(0);
    let restorePresentation: Promise<void> | undefined;
    const restore = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_frozen', selection: saved } });
      restorePresentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'reveal' });
    });
    expect(restore).not.toBeNull();
    if (!restore) throw new Error('Expected a restore update');
    expect(restorePresentation).toBeDefined();

    let restorePresented = false;
    void restorePresentation?.then(() => {
      restorePresented = true;
    });
    await Promise.resolve();
    expect(restorePresented).toBe(false);

    let firstRestoredFrame: WebFrameSample | undefined;
    for (let frame = 0; frame < 60; frame += 1) {
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, frame, null);
      expect(sample.hasNativeFrame).toBe(true);
      expect(sample.caretOnCursorPage).toBe(true);
      expect(sample.actualCursorY).toBeCloseTo(sample.expectedCursorY, 0);
      if (sample.revision >= restore.revision) {
        firstRestoredFrame = sample;
        break;
      }
    }

    expect(firstRestoredFrame, 'restored selection was not presented within 60 frames').toBeDefined();
    await expect(restorePresentation).resolves.toBeUndefined();
    expect(firstRestoredFrame?.scrollTop).toBeGreaterThan(0);
    expect(firstRestoredFrame?.selectionHeadVisible).toBe(true);
  });

  it('expands nested folds before presenting a restored selection', async () => {
    const targetText = 'restored folded caret';
    const { editor } = await mountEditor(nestedFoldedSelectionDoc(targetText));
    const caretOffset = editor.proseText().indexOf(targetText) + targetText.length;
    expect(caretOffset).toBeGreaterThan(targetText.length - 1);
    const selection = editor.proseToSelection(caretOffset, caretOffset);
    const saved = selection && editor.freezeSelection(selection);
    expect(saved).toBeDefined();
    if (!saved) throw new Error('Expected a restorable folded selection');

    const hiddenRestore = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_frozen', selection: saved } });
    });
    expect(hiddenRestore).not.toBeNull();
    if (!hiddenRestore) throw new Error('Expected a hidden-selection restore update');
    await waitForPresentation(editor, hiddenRestore.revision);
    expect(editor.appliedSnapshot.selection).toBeDefined();
    expect(selectionHeadRect(editor.appliedSnapshot)).toBeNull();

    let restorePresentation: Promise<void> | undefined;
    const restore = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_frozen', selection: saved } });
      request.enqueue({ type: 'view', op: { type: 'expand_folds_for_selection' } });
      restorePresentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'reveal' });
    });
    expect(restore).not.toBeNull();
    if (!restore) throw new Error('Expected a folded restore update');

    await expect.poll(() => selectionHeadRect(editor.appliedSnapshot)).not.toBeNull();
    await expect(restorePresentation).resolves.toBeUndefined();
    expect(editor.publishedRevision).toBeGreaterThanOrEqual(restore.revision);
  });

  it('smoothly reveals a spellcheck result after its page surface was virtualized', async () => {
    const pageCount = 8;
    const errorId = 'far-spellcheck-error';
    const href = 'https://example.com/far-spellcheck';
    const { editor, scrollRoot } = await mountEditor(paginatedDocWithPageBreaks(pageCount, { text: 'far typo', href }));

    const farUpdate = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: pageCount - 1, x: PAGE_MARGIN, y: PAGE_MARGIN } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
    expect(farUpdate).not.toBeNull();
    if (!farUpdate) throw new Error('Expected the far-page selection update');
    await waitForPresentation(editor, farUpdate.revision);

    const selection = editor.appliedSnapshot.selection;
    const errorSelection = selection && editor.modifierSpanSelection(selection.head, 'link');
    expect(errorSelection).toBeDefined();
    if (!errorSelection) throw new Error('Expected the linked text to produce a spellcheck range');

    const resetUpdate = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
    expect(resetUpdate).not.toBeNull();
    if (!resetUpdate) throw new Error('Expected the viewport reset update');
    await waitForPresentation(editor, resetUpdate.revision);
    await vi.waitFor(() => expect(editor.published?.frames.has(pageCount - 1)).toBe(false));
    expect(scrollRoot.scrollTop).toBe(0);

    editor.setSpellcheckErrors([
      {
        id: errorId,
        selection: errorSelection,
        context: 'far typo',
        corrections: ['far type'],
        explanation: '',
      },
    ]);
    await expect.poll(() => editor.appliedSnapshot.trackedRanges.some((range) => range.id === errorId)).toBe(true);
    await waitForPresentation(editor);

    editor.setActiveSpellcheckError(errorId);

    let revealed = false;
    for (let frame = 0; frame < 240; frame += 1) {
      await nextAnimationFrame();
      const published = editor.published;
      const range = published?.snapshot.trackedRanges.find((item) => item.id === errorId);
      const targetRect = range && pageRectsToClientRect(editor, range.rects);
      const viewportRect = scrollRoot.getBoundingClientRect();
      if (
        !targetRect ||
        !published?.frames.has(pageCount - 1) ||
        targetRect.top < viewportRect.top - COORDINATE_TOLERANCE_PX ||
        targetRect.bottom > viewportRect.bottom + COORDINATE_TOLERANCE_PX
      ) {
        continue;
      }
      revealed = true;
      break;
    }
    expect(revealed, 'the offscreen spellcheck result was not presented after its smooth reveal').toBe(true);
    expectActualCanvas(editor, pageCount - 1);
  });

  it('keeps an in-flight smooth reveal advancing while preceding page edits are published', async () => {
    const { editor, errorId, presentation, scrollRoot } = await prepareTrackedItemSmoothReveal(32, 10);
    let completed = false;
    void presentation.then(() => {
      completed = true;
    });
    const initialScrollTop = scrollRoot.scrollTop;
    await expect.poll(() => Math.abs(scrollRoot.scrollTop - initialScrollTop)).toBeGreaterThan(0.25);
    const initialTargetScrollTop = trackedTargetScrollTop(editor, errorId);
    expect(initialTargetScrollTop).not.toBeNull();
    if (initialTargetScrollTop === null) throw new Error('Expected initial tracked target geometry');
    let previousDistance = scrollRoot.scrollTop - initialTargetScrollTop;
    let consecutiveStationaryFrames = 0;
    let maximumStationaryFrames = 0;
    let insertedPages = 0;
    const samples: {
      frame: number;
      capturedAtMs: number;
      scrollTop: number;
      targetScrollTop: number | null;
      distance: number | null;
      revision: number;
    }[] = [];

    for (let frame = 0; !completed && frame < 240; frame += 1) {
      if (frame === 2) {
        insertPagesBeforeTrackedTarget(editor, 20);
        insertedPages = 20;
      }
      const capturedAtMs = await nextAnimationFrame();
      const targetScrollTop = trackedTargetScrollTop(editor, errorId);
      const currentScrollTop = scrollRoot.scrollTop;
      const distance = targetScrollTop === null ? null : currentScrollTop - targetScrollTop;
      samples.push({
        frame,
        capturedAtMs,
        scrollTop: currentScrollTop,
        targetScrollTop,
        distance,
        revision: editor.publishedRevision ?? -1,
      });
      if (distance !== null && distance > 20) {
        consecutiveStationaryFrames = Math.abs(distance - previousDistance) <= 0.25 ? consecutiveStationaryFrames + 1 : 0;
        maximumStationaryFrames = Math.max(maximumStationaryFrames, consecutiveStationaryFrames);
        previousDistance = distance;
      }
    }

    expect(insertedPages).toBe(20);
    expect(completed, 'smooth reveal did not complete after preceding page edits').toBe(true);
    expect(
      maximumStationaryFrames,
      `smooth reveal visibly stalled between animation frames: ${JSON.stringify(samples.slice(-20))}`,
    ).toBeLessThanOrEqual(1);
  });

  it('converges within scroll tolerance when preceding page edits arrive near completion', async () => {
    const { editor, errorId, presentation, scrollRoot } = await prepareTrackedItemSmoothReveal(32, 10);
    let completed = false;
    void presentation.then(() => {
      completed = true;
    });
    let insertedPages = false;

    for (let frame = 0; !completed && frame < 240; frame += 1) {
      await nextAnimationFrame();
      const targetScrollTop = trackedTargetScrollTop(editor, errorId);
      if (!insertedPages && targetScrollTop !== null && scrollRoot.scrollTop - targetScrollTop < scrollRoot.clientHeight) {
        insertPagesBeforeTrackedTarget(editor, 20);
        insertedPages = true;
      }
    }

    expect(insertedPages, 'TEST HARNESS: preceding edits were not inserted near completion').toBe(true);
    expect(completed, 'smooth reveal did not complete after the near-finish edits').toBe(true);

    const finalEditRevision = insertPagesBeforeTrackedTarget(editor);
    await waitForPresentation(editor, finalEditRevision);
    const finalTargetScrollTop = trackedTargetScrollTop(editor, errorId);
    expect(finalTargetScrollTop).not.toBeNull();
    if (finalTargetScrollTop === null) throw new Error('Expected final tracked target geometry');
    expect(Math.abs(scrollRoot.scrollTop - finalTargetScrollTop)).toBeLessThanOrEqual(1);
  });

  it('rejects a presentation without the cursor page native frame', () => {
    expect(describeWebFrameMismatch(matchingWebFrameSample({ hasNativeFrame: false }))).toBeDefined();
  });

  it('rejects a repeated-input display frame gap over 250ms', () => {
    const samples = [
      matchingWebFrameSample({ frame: 0, revision: 1, capturedAtMs: 0 }),
      matchingWebFrameSample({ frame: 1, revision: 2, capturedAtMs: 16 }),
      matchingWebFrameSample({ frame: 2, revision: 3, capturedAtMs: 32 }),
      matchingWebFrameSample({ frame: 3, revision: 4, capturedAtMs: 400 }),
    ];

    expect(() => expectWebFrameLiveness(samples, 0)).toThrow();
  });

  it('keeps repeated ArrowUp/ArrowDown navigation on one actual presentation in a long document', async () => {
    const { editor, scrollRoot } = await mountEditor(longDoc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    for (const [phaseIndex, phaseMs] of REPEAT_START_PHASES_MS.entries()) {
      const samples = await driveWebRepeatPhase(
        editor,
        scrollRoot,
        phaseMs,
        repeatKeys('ArrowDown', 'ArrowUp', REPEAT_EVENTS_PER_LEG, REPEAT_CYCLES_PER_PHASE),
        'Arrow-repeat',
      );
      expectWebFrameLiveness(samples, phaseMs);
      expect(
        samples.some((sample) => sample.scrollTop > COORDINATE_TOLERANCE_PX),
        `TEST HARNESS: phase ${phaseMs}ms never scrolled the long document`,
      ).toBe(true);
      if (phaseIndex < REPEAT_START_PHASES_MS.length - 1) {
        await resetWebLongDocumentStart(editor, scrollRoot);
      }
    }
  });

  it('keeps 33ms Enter/Backspace repeats live in a 30000-character document', async () => {
    const { editor, scrollRoot } = await mountEditor(longDoc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    for (const [phaseIndex, phaseMs] of REPEAT_START_PHASES_MS.entries()) {
      const samples = await driveWebRepeatPhase(
        editor,
        scrollRoot,
        phaseMs,
        repeatKeys('Enter', 'Backspace', EDIT_REPEAT_EVENTS_PER_LEG, 1),
        'Enter/Backspace repeat',
      );
      expectWebFrameLiveness(samples, phaseMs);
      expect(
        new Set(samples.map((sample) => sample.documentHeight)).size,
        `TEST HARNESS: phase ${phaseMs}ms never changed the 30000-character document extent`,
      ).toBeGreaterThan(1);
      if (phaseIndex < REPEAT_START_PHASES_MS.length - 1) {
        await resetWebLongDocumentStart(editor, scrollRoot);
      }
    }
  }, 30_000);

  it('keeps empty-document Enter/Backspace page transitions on one actual presentation', async () => {
    const { editor, scrollRoot } = await mountEditor(doc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    for (let i = 0; i < 32 && editor.appliedSnapshot.pageSizes.length === 1; i += 1) await pressEditorKey(editor, 'Enter');

    expect(editor.appliedSnapshot.pageSizes).toHaveLength(2);
    const crossing = editor.published;
    expect(
      crossing?.snapshot.cursor?.page_idx !== 1 || crossing?.frames.has(1) === true,
      `published snapshot ${crossing?.snapshot.revision} exposed cursor page 1 before that page's actual canvas frame was present`,
    ).toBe(true);

    await waitForPresentation(editor);
    expect(editor.published?.snapshot.cursor?.page_idx).toBe(1);
    expectActualCanvas(editor, 1, false);
    expect(
      scrollRoot.scrollTop,
      `scrollTop=${scrollRoot.scrollTop} clientHeight=${scrollRoot.clientHeight} scrollHeight=${scrollRoot.scrollHeight} cursor=${JSON.stringify(editor.published?.snapshot.cursor)}`,
    ).toBeGreaterThan(0);
    expectSelectionHeadVisible(editor, scrollRoot, 'initial 1→2 page crossing');

    for (let cycle = 0; cycle < 3; cycle += 1) {
      await pressEditorKey(editor, 'Backspace');
      expect(editor.appliedSnapshot.pageSizes).toHaveLength(1);
      const shrinking = editor.published;
      expect(
        shrinking?.snapshot.pageSizes.length !== 1 || (shrinking.snapshot.cursor?.page_idx === 0 && shrinking.frames.has(0)),
        `cycle ${cycle} published the 2→1 geometry without page 0's cursor and actual frame`,
      ).toBe(true);
      await waitForPresentation(editor);
      expect(editor.published?.snapshot.pageSizes).toHaveLength(1);
      expectActualCanvas(editor, 0, false);

      if (cycle === 0) {
        const beforeRevision = editor.appliedRevision;
        dispatchEditorKey(editor, 'Enter');
        dispatchEditorKey(editor, 'Backspace');
        await expect.poll(() => editor.appliedRevision).toBeGreaterThanOrEqual(beforeRevision + 2);
        expect(editor.appliedSnapshot.pageSizes).toHaveLength(1);
        await waitForPresentation(editor);
        expect(editor.published?.snapshot.pageSizes).toHaveLength(1);
        expectActualCanvas(editor, 0, false);
      }

      await pressEditorKey(editor, 'Enter');
      expect(editor.appliedSnapshot.pageSizes).toHaveLength(2);
      const repeated = editor.published;
      expect(
        repeated?.snapshot.cursor?.page_idx !== 1 || repeated?.frames.has(1) === true,
        `cycle ${cycle} published page 1 geometry without its actual frame`,
      ).toBe(true);
      await waitForPresentation(editor);
      expect(editor.published?.snapshot.pageSizes).toHaveLength(2);
      expectActualCanvas(editor, 1, false);
      expectSelectionHeadVisible(editor, scrollRoot, `cycle ${cycle} 1→2 page crossing`);
    }
  });

  it('presents coalesced 1→3 growth and 3→1 shrink through the production input path', async () => {
    const { editor, scrollRoot } = await mountEditor(doc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    for (let event = 0; event < 128 && editor.appliedSnapshot.pageSizes.length < 3; event += 1) {
      dispatchEditorKey(editor, 'Enter');
    }
    expect(editor.appliedSnapshot.pageSizes).toHaveLength(3);
    expect(editor.published?.snapshot.pageSizes).toHaveLength(1);
    const growthRevision = editor.appliedRevision;

    let growthFrame: WebFrameSample | undefined;
    for (let frame = 0; frame < 60; frame += 1) {
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, frame, null);
      const mismatch = describeWebFrameMismatch(sample);
      expect(mismatch, mismatch).toBeUndefined();
      if (sample.revision >= growthRevision) {
        growthFrame = sample;
        break;
      }
    }
    expect(growthFrame, 'the coalesced 1→3 revision was not presented within 60 frames').toBeDefined();
    expect(editor.published?.snapshot.pageSizes).toHaveLength(3);
    expect(growthFrame?.cursorPage).toBe(2);
    expectActualCanvas(editor, 2, false);
    expectSelectionHeadVisible(editor, scrollRoot, 'coalesced 1→3 growth');

    for (let event = 0; event < 128 && editor.appliedSnapshot.pageSizes.length > 1; event += 1) {
      dispatchEditorKey(editor, 'Backspace');
    }
    expect(editor.appliedSnapshot.pageSizes).toHaveLength(1);
    expect(editor.published?.snapshot.pageSizes).toHaveLength(3);
    const shrinkRevision = editor.appliedRevision;

    let shrinkFrame: WebFrameSample | undefined;
    for (let frame = 0; frame < 60; frame += 1) {
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, frame, null);
      const mismatch = describeWebFrameMismatch(sample);
      expect(mismatch, mismatch).toBeUndefined();
      if (sample.revision >= shrinkRevision) {
        shrinkFrame = sample;
        break;
      }
    }
    expect(shrinkFrame, 'the coalesced 3→1 revision was not presented within 60 frames').toBeDefined();
    expect(editor.published?.snapshot.pageSizes).toHaveLength(1);
    expect(shrinkFrame?.cursorPage).toBe(0);
    expectActualCanvas(editor, 0, false);
    expect(document.querySelector('[data-page-canvas="1"]')).toBeNull();
    expect(document.querySelector('[data-page-canvas="2"]')).toBeNull();
  });

  it('keeps typewriter reveal aligned for Enter edits within an existing paginated canvas', async () => {
    const { editor, scrollRoot } = await mountEditor(doc(), { typewriterEnabled: true });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const samples = await driveWebRepeatPhase(
      editor,
      scrollRoot,
      0,
      Array.from({ length: EDIT_REPEAT_EVENTS_PER_LEG }, () => 'Enter'),
      'same-canvas paginated Enter repeat',
    );
    expectWebFrameLiveness(samples, 0);
    const stableExtentEdits = samples.filter(
      (sample, index) =>
        index > 0 &&
        sample.revision > samples[index - 1].revision &&
        sample.documentHeight === samples[index - 1].documentHeight &&
        sample.scrollTop > 0,
    );
    expect(
      stableExtentEdits.length,
      'TEST HARNESS: no presented Enter edit stayed within an existing canvas after scrolling',
    ).toBeGreaterThan(0);
    const cursorHeight = (editor.published?.snapshot.cursor?.caret.height ?? 0) * editor.safeDisplayZoom();
    const expectedCursorY = scrollRoot.getBoundingClientRect().top + (scrollRoot.clientHeight - cursorHeight) / 2;
    for (const sample of stableExtentEdits) {
      expect(
        sample.actualCursorY !== null && Math.abs(sample.actualCursorY - expectedCursorY) <= COORDINATE_TOLERANCE_PX,
        `frame ${sample.frame}: cursor=${sample.actualCursorY} expected typewriter position=${expectedCursorY} scrollTop=${sample.scrollTop}`,
      ).toBe(true);
    }
  });

  it('reveals a page break inserted while composing on its first published frame', async () => {
    const { editor, scrollRoot } = await mountEditor(doc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const input = editor.inputEl;
    if (!input) throw new Error('Production editor input is not mounted');
    input.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));
    input.dispatchEvent(new CompositionEvent('compositionupdate', { data: '가', bubbles: true }));
    input.dispatchEvent(
      new InputEvent('beforeinput', {
        inputType: 'insertCompositionText',
        data: '가',
        isComposing: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    input.value = '\u{2028}가\u{2029}';
    input.setSelectionRange(2, 2);
    input.dispatchEvent(new InputEvent('input', { inputType: 'insertCompositionText', data: '가', isComposing: true, bubbles: true }));

    const beforeRevision = dispatchEditorKey(
      editor,
      'Enter',
      navigator.platform.toUpperCase().includes('MAC') ? { metaKey: true, isComposing: true } : { ctrlKey: true, isComposing: true },
    );
    input.dispatchEvent(new CompositionEvent('compositionend', { data: '가', bubbles: true }));
    expect(() => editor.ime(4096, 4096)).not.toThrow();
    let firstInsertedFrame: WebFrameSample | undefined;
    for (let frame = 0; frame < 60; frame += 1) {
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, frame, null);
      const mismatch = describeWebFrameMismatch(sample);
      expect(mismatch, mismatch).toBeUndefined();
      if (sample.revision > beforeRevision) {
        firstInsertedFrame = sample;
        break;
      }
    }

    expect(firstInsertedFrame, 'page-break revision was not presented within 60 frames').toBeDefined();
    expect(editor.published?.snapshot.pageSizes).toHaveLength(2);
    expect(firstInsertedFrame?.cursorPage).toBe(1);
    expect(firstInsertedFrame?.scrollTop).toBeGreaterThan(0);
    expectActualCanvas(editor, 1, false);
  });

  it('keeps the published texture visible through display and render zoom changes', async () => {
    const { editor, scrollRoot } = await mountEditor(doc('linked pixels', 'https://example.com/zoom'), { withZoom: true });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();
    const displayed = editor.published?.frames.get(0)?.canvas;
    expect(displayed).toBeDefined();
    expect(displayed?.isConnected).toBe(true);
    expectActualCanvas(editor, 0);
    const attachSurface = editor.attachSurface.bind(editor);
    let injectedPresentation = false;
    const attachSurfaceSpy = vi.spyOn(editor, 'attachSurface').mockImplementation((...args) => {
      const result = attachSurface(...args);
      if (!injectedPresentation && args[0] === 0) {
        injectedPresentation = true;
        editor.requestPublication();
      }
      return result;
    });
    let previousSample: WebFrameSample | undefined;

    for (let event = 0; event < 12; event += 1) {
      scrollRoot.dispatchEvent(
        new WheelEvent('wheel', {
          deltaY: event < 9 ? 8 : -8,
          metaKey: navigator.platform.toUpperCase().includes('MAC'),
          ctrlKey: !navigator.platform.toUpperCase().includes('MAC'),
          bubbles: true,
          cancelable: true,
        }),
      );
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, event, null);
      const mismatch = describeWebFrameMismatch(sample, previousSample);
      expect(mismatch, mismatch).toBeUndefined();
      previousSample = sample;
      expect(editor.published?.frames.get(0)?.canvas.isConnected).toBe(true);
      expectActualCanvas(editor, 0);
    }

    await expect.poll(() => Math.abs(editor.renderZoom - editor.displayZoom)).toBeLessThan(0.005);
    for (let frame = 0; frame < 60 && editor.published?.frames.get(0)?.canvas === displayed; frame += 1) {
      await nextAnimationFrame();
      const sample = readWebFrameSample(editor, scrollRoot, 0, 12 + frame, null);
      const mismatch = describeWebFrameMismatch(sample, previousSample);
      expect(mismatch, mismatch).toBeUndefined();
      previousSample = sample;
      expect(editor.published?.frames.get(0)?.canvas.isConnected).toBe(true);
    }
    await waitForPresentation(editor);
    expect(editor.published?.frames.get(0)?.canvas).not.toBe(displayed);
    expect(attachSurfaceSpy.mock.calls.filter(([page]) => page === 0).length).toBeGreaterThan(0);
    expect(editor.published?.frames.get(0)?.canvas.isConnected).toBe(true);
    expectActualCanvas(editor, 0);
  });

  it('stops zoom on the first pinch pointer release and keeps the survivor zoom-owned until all-up', async () => {
    const { editor, scrollRoot } = await mountEditor(doc('touch zoom'), { withZoom: true });
    await waitForPresentation(editor);
    const pageRect = editor.pageEls[0]?.getBoundingClientRect();
    if (!pageRect) throw new Error('Expected a mounted editor page');
    const centerX = pageRect.left + pageRect.width / 2;
    const centerY = pageRect.top + 60;

    dispatchEditorTouch(scrollRoot, 'touchstart', [
      editorTouch(scrollRoot, 0, centerX - 30, centerY),
      editorTouch(scrollRoot, 1, centerX + 30, centerY),
    ]);
    await nextAnimationFrame();
    const survivor = editorTouch(scrollRoot, 0, centerX - 36, centerY);
    dispatchEditorTouch(scrollRoot, 'touchmove', [survivor, editorTouch(scrollRoot, 1, centerX + 36, centerY)]);
    await nextAnimationFrame();
    const zoomAtRelease = editor.displayZoom;

    dispatchEditorTouch(scrollRoot, 'touchend', [survivor]);
    const survivorMove = dispatchEditorTouch(scrollRoot, 'touchmove', [editorTouch(scrollRoot, 0, centerX - 38, centerY)]);

    expect(survivorMove.defaultPrevented).toBe(true);
    await nextAnimationFrame();
    await nextAnimationFrame();
    expect(editor.displayZoom).toBeCloseTo(zoomAtRelease);

    dispatchEditorTouch(scrollRoot, 'touchend', []);
    const nextTouch = editorTouch(scrollRoot, 2, centerX, centerY);
    dispatchEditorTouch(scrollRoot, 'touchstart', [nextTouch]);
    const nextMove = dispatchEditorTouch(scrollRoot, 'touchmove', [editorTouch(scrollRoot, 2, centerX, centerY + 2)]);
    expect(nextMove.defaultPrevented).toBe(false);
    dispatchEditorTouch(scrollRoot, 'touchend', []);
  });

  it('keeps the scrollbar thumb synchronized without showing scroll progress for zoom correction', async () => {
    const { editor, scrollRoot } = await mountEditor(longDoc(), { withZoom: true });
    await waitForPresentation(editor);

    scrollRoot.scrollTop = scrollRoot.scrollHeight / 3;
    scrollRoot.dispatchEvent(new Event('scroll'));
    await tick();

    const verticalScrollbar = document.querySelector<HTMLElement>('[role="scrollbar"]:not([aria-orientation])');
    const thumb = verticalScrollbar?.querySelector<HTMLElement>('[role="slider"]');
    const indicator = verticalScrollbar?.previousElementSibling as HTMLElement | null;
    expect(verticalScrollbar).not.toBeNull();
    expect(thumb).not.toBeNull();
    expect(indicator?.textContent).toMatch(/^\d+%$/);
    if (!thumb || !indicator) throw new Error('Expected the vertical scrollbar thumb and indicator');
    await expect.poll(() => Number.parseFloat(getComputedStyle(indicator).opacity)).toBeGreaterThan(0.99);

    const scrollTopBeforeZoom = scrollRoot.scrollTop;
    const zoomBefore = editor.displayZoom;
    const viewportRect = scrollRoot.getBoundingClientRect();
    scrollRoot.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: -24,
        metaKey: navigator.platform.toUpperCase().includes('MAC'),
        ctrlKey: !navigator.platform.toUpperCase().includes('MAC'),
        clientX: viewportRect.left + viewportRect.width / 2,
        clientY: viewportRect.top + viewportRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );

    await expect.poll(() => editor.displayZoom).toBeGreaterThan(zoomBefore);
    await expect.poll(() => Math.abs(scrollRoot.scrollTop - scrollTopBeforeZoom)).toBeGreaterThan(0.5);
    await expect.poll(() => Number(thumb.getAttribute('aria-valuenow'))).toBeCloseTo(scrollRoot.scrollTop);
    await expect.poll(() => Number.parseFloat(getComputedStyle(indicator).opacity)).toBeLessThan(0.01);
  });

  it('delays continuous reflow until the render commit and replaces it coherently', async () => {
    const { editor, scrollRoot } = await mountEditor(continuousDoc('continuous zoom '.repeat(40)), { withZoom: true });
    await waitForPresentation(editor);
    const attachSurface = editor.attachSurface.bind(editor);
    const attachSurfaceSpy = vi.spyOn(editor, 'attachSurface').mockImplementation((...args) => attachSurface(...args));
    const initialPageWidth = editor.appliedSnapshot.pageSizes[0]?.width;
    const initialCanvas = editor.published?.frames.get(0)?.canvas;
    expect(initialPageWidth).toBeCloseTo(360);
    expect(initialCanvas).toBeDefined();
    expectActualCanvas(editor, 0, false);

    const viewportRect = scrollRoot.getBoundingClientRect();
    scrollRoot.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: 24,
        metaKey: navigator.platform.toUpperCase().includes('MAC'),
        ctrlKey: !navigator.platform.toUpperCase().includes('MAC'),
        clientX: viewportRect.left + viewportRect.width / 2,
        clientY: viewportRect.top + viewportRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    await nextAnimationFrame();

    expect(editor.displayZoom).toBeLessThan(1);
    expect(editor.renderZoom).toBe(1);
    expect(editor.viewport.width).toBeCloseTo(360);
    expect(editor.appliedSnapshot.pageSizes[0]?.width).toBeCloseTo(initialPageWidth ?? 0);
    expect(editor.published?.frames.get(0)?.canvas).toBe(initialCanvas);

    await expect.poll(() => editor.renderZoom).toBeCloseTo(editor.displayZoom);
    await expect.poll(() => editor.viewport.width).toBeGreaterThan(360);
    await waitForPresentation(editor);

    expect(editor.appliedSnapshot.pageSizes[0]?.width).toBeGreaterThan(initialPageWidth ?? Infinity);
    expect(editor.published?.frames.get(0)?.canvas).not.toBe(initialCanvas);
    expect(editor.pageEls[0]?.getBoundingClientRect().width).toBeCloseTo(360, 0);
    expect(editor.published?.frames.get(0)?.canvas.width).toBeGreaterThan(0);
    expect(editor.published?.frames.get(0)?.canvas.height).toBeGreaterThan(0);
    expect(attachSurfaceSpy.mock.calls.filter(([page]) => page === 0)).toHaveLength(1);
    expectActualCanvas(editor, 0, false);
  });

  it('commits a large continuous scale gap with coherent layout and surface publication', async () => {
    const { editor, scrollRoot } = await mountEditor(continuousDoc('continuous threshold zoom '.repeat(40)), { withZoom: true });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();
    const initialPageWidth = editor.appliedSnapshot.pageSizes[0]?.width;
    const initialCanvas = editor.published?.frames.get(0)?.canvas;
    expect(initialPageWidth).toBeCloseTo(360);
    expect(initialCanvas).toBeDefined();
    const initialSample = readWebFrameSample(editor, scrollRoot, 0, -1, null);
    const initialMismatch = describeWebFrameMismatch(initialSample);
    expect(initialMismatch, initialMismatch).toBeUndefined();

    const viewportRect = scrollRoot.getBoundingClientRect();
    scrollRoot.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: 69,
        metaKey: navigator.platform.toUpperCase().includes('MAC'),
        ctrlKey: !navigator.platform.toUpperCase().includes('MAC'),
        clientX: viewportRect.left + viewportRect.width / 2,
        clientY: viewportRect.top + viewportRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    await nextAnimationFrame();

    expect(editor.displayZoom).toBeLessThan(1);
    expect(editor.renderZoom).toBeCloseTo(editor.displayZoom);
    expect(editor.viewport.width).toBeGreaterThan(360);
    expect(editor.appliedSnapshot.pageSizes[0]?.width).toBeGreaterThan(initialPageWidth ?? Infinity);
    const intermediateSample = readWebFrameSample(editor, scrollRoot, 0, 0, null);
    const intermediateMismatch = describeWebFrameMismatch(intermediateSample, initialSample);
    expect(intermediateMismatch, intermediateMismatch).toBeUndefined();
    expect(editor.published?.frames.get(0)?.canvas.isConnected).toBe(true);
    expectActualCanvas(editor, 0, false);

    await waitForPresentation(editor);

    const settledSample = readWebFrameSample(editor, scrollRoot, 0, 1, null);
    const settledMismatch = describeWebFrameMismatch(settledSample, intermediateSample);
    expect(settledMismatch, settledMismatch).toBeUndefined();
    expect(editor.published?.frames.get(0)?.canvas).not.toBe(initialCanvas);
    expect(editor.pageEls[0]?.getBoundingClientRect().width).toBeCloseTo(360, 0);
    expectActualCanvas(editor, 0, false);
  });

  it('derives a continuous render commit from the editor viewport target', async () => {
    const { editor } = await mountEditor(continuousDoc('continuous zoom '.repeat(40)));
    await waitForPresentation(editor);

    editor.resizeViewportNow(300, 180, 1);
    await waitForPresentation(editor);
    expect(editor.viewport.width).toBeCloseTo(300);

    editor.commitRenderZoom(0.75);

    expect(editor.viewport.width).toBeCloseTo(400);
    expect(editor.appliedSnapshot.pageSizes[0]?.width).toBeCloseTo(400);
  });

  it('makes the entire zoomed continuous track horizontally reachable without oversized line highlight paint', async () => {
    const { editor, extensionArea, scrollRoot } = await mountEditor(continuousDoc('continuous zoom'), { withZoom: true });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const viewportRect = scrollRoot.getBoundingClientRect();
    scrollRoot.dispatchEvent(
      new WheelEvent('wheel', {
        deltaY: -98,
        metaKey: navigator.platform.toUpperCase().includes('MAC'),
        ctrlKey: !navigator.platform.toUpperCase().includes('MAC'),
        clientX: viewportRect.left + viewportRect.width / 2,
        clientY: viewportRect.top + viewportRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    await nextAnimationFrame();
    expect(editor.displayZoom).toBeGreaterThan(1);

    scrollRoot.scrollLeft = 0;
    scrollRoot.dispatchEvent(new Event('scroll'));
    await nextAnimationFrame();

    const pageElement = editor.pageEls[0];
    const documentTrack = document.querySelector<HTMLElement>('[data-editor-document-track]');
    const lineHighlight = document.querySelector<HTMLElement>('[data-editor-line-highlight]');
    expect(pageElement).toBeDefined();
    expect(documentTrack).not.toBeNull();
    expect(lineHighlight?.parentElement).toBe(extensionArea);
    if (!pageElement || !documentTrack || !lineHighlight) throw new Error('Expected continuous zoom geometry');

    const pageRectAtStart = pageElement.parentElement?.getBoundingClientRect();
    const trackRectAtStart = documentTrack.getBoundingClientRect();
    expect(pageRectAtStart?.left).toBeCloseTo(viewportRect.left, 0);
    expect(trackRectAtStart.left).toBeCloseTo(viewportRect.left, 0);
    expect(trackRectAtStart.width).toBeCloseTo(pageRectAtStart?.width ?? 0, 0);
    expect(scrollRoot.scrollWidth).toBeCloseTo(trackRectAtStart.width, 0);
    expect(lineHighlight.getBoundingClientRect().width).toBeCloseTo(extensionArea.getBoundingClientRect().width, 0);
    expect(getComputedStyle(lineHighlight).boxShadow).toBe('none');

    const maximumScrollLeft = scrollRoot.scrollWidth - scrollRoot.clientWidth;
    expect(maximumScrollLeft).toBeGreaterThan(0);
    const horizontalScrollbarThumb = document.querySelector<HTMLElement>(
      '[role="scrollbar"][aria-orientation="horizontal"] [role="slider"]',
    );
    expect(horizontalScrollbarThumb).not.toBeNull();
    if (!horizontalScrollbarThumb) throw new Error('Expected the horizontal scrollbar thumb');
    const thumbRect = horizontalScrollbarThumb.getBoundingClientRect();
    const pointerId = 1;
    horizontalScrollbarThumb.dispatchEvent(
      new PointerEvent('pointerdown', {
        pointerId,
        pointerType: 'mouse',
        isPrimary: true,
        button: 0,
        buttons: 1,
        clientX: thumbRect.left + thumbRect.width / 2,
        clientY: thumbRect.top + thumbRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    horizontalScrollbarThumb.dispatchEvent(
      new PointerEvent('pointermove', {
        pointerId,
        pointerType: 'mouse',
        isPrimary: true,
        buttons: 1,
        clientX: thumbRect.left + thumbRect.width / 2 + 40,
        clientY: thumbRect.top + thumbRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    horizontalScrollbarThumb.dispatchEvent(
      new PointerEvent('pointerup', {
        pointerId,
        pointerType: 'mouse',
        isPrimary: true,
        button: 0,
        clientX: thumbRect.left + thumbRect.width / 2 + 40,
        clientY: thumbRect.top + thumbRect.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    );
    await nextAnimationFrame();
    expect(scrollRoot.scrollLeft).toBeGreaterThan(0);
    const draggedScrollLeft = scrollRoot.scrollLeft;
    scrollRoot.dispatchEvent(new Event('scroll'));
    await tick();
    await nextAnimationFrame();
    await nextAnimationFrame();
    expect(scrollRoot.scrollLeft).toBeCloseTo(draggedScrollLeft, 0);

    scrollRoot.scrollLeft = maximumScrollLeft;
    scrollRoot.dispatchEvent(new Event('scroll'));
    await nextAnimationFrame();

    expect(scrollRoot.scrollLeft).toBeCloseTo(maximumScrollLeft, 0);
    expect(documentTrack.getBoundingClientRect().right).toBeCloseTo(viewportRect.right, 0);
  });

  it('fills the continuous editor surface behind reserved review-column insets', async () => {
    const { editor, extensionArea } = await mountEditor(continuousDoc('continuous review column'), {
      contentInsetLeft: 100,
      contentInsetRight: 396,
    });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const lineHighlight = document.querySelector<HTMLElement>('[data-editor-line-highlight]');
    expect(lineHighlight).not.toBeNull();
    if (!lineHighlight) throw new Error('Expected continuous line highlight');

    const surfaceRect = extensionArea.getBoundingClientRect();
    const highlightRect = lineHighlight.getBoundingClientRect();
    expect(lineHighlight.parentElement).toBe(extensionArea);
    expect(highlightRect.left).toBeCloseTo(surfaceRect.left, 0);
    expect(highlightRect.right).toBeCloseTo(surfaceRect.right, 0);
    expect(getComputedStyle(lineHighlight).boxShadow).toBe('none');
  });

  it('prepares an appended page without mounting candidate geometry before publication', async () => {
    const { editor, extensionArea, scrollRoot } = await mountEditor(doc());
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const initialScrollHeight = scrollRoot.scrollHeight;
    const initialExtensionRect = extensionArea.getBoundingClientRect();
    const initialPageRect = editor.pageEls[0]?.getBoundingClientRect();
    expect(initialPageRect).toBeDefined();
    if (!initialPageRect) throw new Error('Expected the initial published page');

    const initialPageTop = initialPageRect.top - initialExtensionRect.top;
    const attachSurface = editor.attachSurface.bind(editor);
    const attachSurfaceSpy = vi.spyOn(editor, 'attachSurface').mockImplementation((page, canvas, width, height, replace) => {
      if (page === 1) return 'none';
      return attachSurface(page, canvas, width, height, replace);
    });

    try {
      for (let i = 0; i < 32 && editor.appliedSnapshot.pageSizes.length === 1; i += 1) await pressEditorKey(editor, 'Enter');
      expect(editor.appliedSnapshot.pageSizes).toHaveLength(2);
      await vi.waitFor(() => expect(editor.surfacePageRequirements.has(1)).toBe(true));
      const stalledPublished = editor.published;
      expect(stalledPublished?.snapshot.pageSizes).toHaveLength(1);
      if (!stalledPublished) throw new Error('Expected the previous publication during page preparation');
      await tick();
      await nextAnimationFrame();

      const currentExtensionRect = extensionArea.getBoundingClientRect();
      const currentPageRect = editor.pageEls[0]?.getBoundingClientRect();
      expect(editor.pageEls[1]).toBeUndefined();
      expect(currentPageRect).toBeDefined();
      if (!currentPageRect) throw new Error('Expected the published page to remain mounted during preparation');
      expect(editor.published?.snapshot.revision).toBe(stalledPublished.snapshot.revision);
      expect(scrollRoot.scrollHeight).toBe(initialScrollHeight);
      expect(currentPageRect.top - currentExtensionRect.top).toBeCloseTo(initialPageTop);
      expect(currentPageRect.width).toBeCloseTo(initialPageRect.width);
      expect(currentPageRect.height).toBeCloseTo(initialPageRect.height);
    } finally {
      attachSurfaceSpy.mockRestore();
    }

    window.dispatchEvent(new Event('pageshow'));
    await waitForPresentation(editor);
    expect(editor.published?.snapshot.pageSizes).toHaveLength(2);
    expect(scrollRoot.scrollHeight).toBeGreaterThan(initialScrollHeight);
  });

  it('keeps canvas, cursor, current line, page overlay, and fixed input on the published geometry', async () => {
    const href = 'https://example.com/frame-sync';
    const { editor, scrollRoot } = await mountEditor(doc('linked pixels', href));
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    expectActualCanvas(editor, 0);
    const pageElement = editor.pageEls[0];
    const caret = document.querySelector<HTMLElement>('[data-editor-caret]');
    const lineHighlight = document.querySelector<HTMLElement>('[data-editor-line-highlight]');
    const input = editor.inputEl;
    expect(pageElement).toBeDefined();
    expect(caret?.parentElement).toBe(pageElement);
    expect(lineHighlight?.parentElement).toBe(pageElement);
    await expect.poll(() => pageElement?.querySelector<HTMLAnchorElement>(`a[aria-label="${href}"]`) !== null).toBe(true);
    const linkOverlay = pageElement?.querySelector<HTMLAnchorElement>(`a[aria-label="${href}"]`);
    expect(getComputedStyle(caret as HTMLElement).visibility).toBe('visible');
    expect(getComputedStyle(lineHighlight as HTMLElement).display).toBe('block');
    const linkRect = editor.pageLinkRects(0).find((link) => link.href === href)?.rects[0];
    expect(linkRect).toBeDefined();
    expect(Number.parseFloat(linkOverlay?.style.left ?? '')).toBeCloseTo(linkRect?.x ?? NaN);
    expect(Number.parseFloat(linkOverlay?.style.top ?? '')).toBeCloseTo(linkRect?.y ?? NaN);
    expect(linkOverlay?.parentElement).toBe(pageElement);

    const cursor = editor.published?.snapshot.cursor;
    expect(cursor).toBeDefined();
    await expect
      .poll(() => {
        const current = editor.published?.snapshot.cursor;
        const expected = current && pageRectToClientRect(editor, { page_idx: current.page_idx, rect: current.caret });
        return expected && input
          ? Math.abs(Number.parseFloat(input.style.left) - expected.left) <= 0.5 &&
              Math.abs(Number.parseFloat(input.style.top) - expected.top) <= 0.5
          : false;
      })
      .toBe(true);

    const screenshot = await page.screenshot({ element: scrollRoot, save: false });
    expect(screenshot.startsWith('iVBOR')).toBe(true);
  });

  it('hides document overlays and interaction when the published cursor page has no frame', async () => {
    const href = 'https://example.com/frameless-page';
    const { editor } = await mountEditor(doc('linked pixels', href));
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    editor.focus();
    await tick();

    const bundle = editor.published;
    const pageElement = editor.pageEls[0];
    expect(bundle).toBeDefined();
    expect(pageElement).toBeDefined();
    if (!bundle || !pageElement) throw new Error('Expected a framed published page');
    const pageRect = pageElement.getBoundingClientRect();

    editor.published = { snapshot: bundle.snapshot, frames: new Map() };
    await tick();

    const caret = document.querySelector<HTMLElement>('[data-editor-caret]');
    const lineHighlight = document.querySelector<HTMLElement>('[data-editor-line-highlight]');
    expect(getComputedStyle(caret as HTMLElement).visibility).toBe('hidden');
    expect(getComputedStyle(lineHighlight as HTMLElement).display).toBe('none');
    expect(editor.inputEl?.style.left).toBe('-9999px');
    expect(pageElement.querySelector(`a[aria-label="${href}"]`)).toBeNull();
    expect(editor.clientToLocal(pageRect.left + PAGE_MARGIN, pageRect.top + PAGE_MARGIN)).toBeNull();
  });

  it('keeps non-vacuous fixed selection handles on the published page and scroll geometry', async () => {
    const href = 'https://example.com/selection-handles';
    const { editor, scrollRoot } = await mountEditor(doc('select this link', href), { readOnly: true });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_MARGIN } }));
    await waitForPresentation(editor);
    const selection = editor.selection;
    const span = selection && editor.modifierSpanSelection(selection.head, 'link');
    expect(span).toBeDefined();
    if (!span) throw new Error('Expected the linked text to produce an expanded selection');
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set', selection: span } }));
    await waitForPresentation(editor);

    scrollRoot.scrollTop = 24;
    scrollRoot.dispatchEvent(new Event('scroll'));
    editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

    const endpoints = editor.selectionEndpoints();
    const fromHandle = document.querySelector<HTMLButtonElement>('[data-selection-handle="from"]');
    const toHandle = document.querySelector<HTMLButtonElement>('[data-selection-handle="to"]');
    expect(endpoints).toBeDefined();
    expect(fromHandle).toBeDefined();
    expect(toHandle).toBeDefined();
    if (!endpoints || !fromHandle || !toHandle) throw new Error('Expected both production selection handles');

    for (const [kind, endpoint, handle] of [
      ['from', endpoints.from, fromHandle],
      ['to', endpoints.to, toHandle],
    ] as const) {
      const anchorRect = pageRectToClientRect(editor, endpoint);
      expect(anchorRect).toBeDefined();
      if (!anchorRect) throw new Error(`Expected the ${kind} endpoint to resolve to client geometry`);
      const visual = computeSelectionHandleVisual({ kind, anchorRect });
      expect(getComputedStyle(handle).position).toBe('fixed');
      expect(Number.parseFloat(handle.style.left)).toBeCloseTo(visual.left);
      expect(Number.parseFloat(handle.style.top)).toBeCloseTo(visual.top);
    }

    const bundle = editor.published;
    if (!bundle) throw new Error('Expected a published selection bundle');
    editor.published = { snapshot: bundle.snapshot, frames: new Map() };
    await tick();
    expect(document.querySelector('[data-selection-handle="from"]')).toBeNull();
    expect(document.querySelector('[data-selection-handle="to"]')).toBeNull();
  });

  it('keeps cursor-guard bottom padding when typewriter mode is disabled', async () => {
    const { editor, extensionArea } = await mountEditor(doc(), { typewriterEnabled: false });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: PAGE_MARGIN, y: PAGE_HEIGHT } }));
    await waitForPresentation(editor);

    expect(Number.parseFloat(extensionArea.style.paddingBottom)).toBeGreaterThanOrEqual(60);
  });

  it('keeps the cursor guard stable after fractional page heights accumulate', async () => {
    const pageCount = 32;
    const { editor, scrollRoot } = await mountEditor(paginatedDocWithPageBreaks(pageCount), {
      displayZoom: 0.75,
      typewriterEnabled: false,
    });
    expect(editor.published?.snapshot.pageSizes).toHaveLength(pageCount);

    const update = editor.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: pageCount - 1, x: PAGE_MARGIN, y: 1_000_000 } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
    expect(update).not.toBeNull();
    if (!update) throw new Error('Expected the far-page reveal update');
    await waitForPresentation(editor, update.revision);

    const head = selectionHeadRect(editor.published?.snapshot);
    const headRect = head && pageRectToClientRect(editor, head);
    expect(headRect).not.toBeNull();
    if (!headRect) throw new Error('Expected the presented selection head geometry');
    const viewportRect = scrollRoot.getBoundingClientRect();
    expect(viewportRect.bottom - headRect.bottom).toBeGreaterThanOrEqual(CURSOR_VISIBLE_MARGIN - COORDINATE_TOLERANCE_PX);
  });
});
