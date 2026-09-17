import '../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import { resolveCachedPageSpans } from './geometry';
import { readNativeSelection } from './native-selection';
import { loadSelectionFonts } from './native-selection-layout';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';
import type { EditorFrameSyncTestHarness } from './editor-frame-sync-test-host.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@mearie/svelte', async (importOriginal) => {
  const original = await importOriginal<typeof import('@mearie/svelte')>();
  return { ...original, createMutation: () => [vi.fn()] };
});
function defined<T>(value: T | null | undefined): T {
  if (value === null || value === undefined) throw new Error('Missing selection test fixture');
  return value;
}

let mounted: Record<string, unknown> | undefined;
let editor: Editor;
afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  editor?.destroy();
  document.body.replaceChildren();
  window.getSelection()?.removeAllRanges();
});
const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({ node, children, modifiers: {} as never, carry: [] });
const paragraph = (text: string) => entry({ type: 'paragraph' }, [entry({ type: 'text', text })]);
const doc = (children: PlainNodeEntry[]): PlainDoc => ({
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, children),
});
async function viewer(plain: PlainDoc) {
  const started = performance.now();
  editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  const engineReady = performance.now();
  editor.readOnly = true;
  editor.nativeSelection = true;
  const layoutReady = performance.now();
  const ready = Promise.withResolvers<EditorFrameSyncTestHarness>();
  const target = document.createElement('div');
  document.body.append(target);
  mounted = mount(EditorFrameSyncTestHost, {
    target,
    props: { editor, readOnly: true, nativeSelection: true, onReady: ready.resolve, userId: `native-${crypto.randomUUID()}` },
  });
  const harness = await ready.promise;
  await tick();
  await vi.waitFor(() => expect(document.querySelectorAll('[data-selection-run]').length).toBeGreaterThan(0));
  const root = defined(document.querySelector<HTMLElement>('[data-native-selection-layer]'));
  await vi.waitFor(() => expect(root.classList.contains('fonts-ready')).toBe(true));
  return {
    root,
    ...harness,
    timings: { engineMs: engineReady - started, exportMs: layoutReady - engineReady, mountMs: performance.now() - layoutReady },
  };
}
function select(root: HTMLElement, first: number, last: number, start: number, end: number, backwards = false) {
  const spans = root.querySelectorAll('[data-selection-run]');
  const a = defined(spans[first].firstChild);
  const b = defined(spans[last].firstChild);
  defined(window.getSelection()).setBaseAndExtent(backwards ? b : a, backwards ? end : start, backwards ? a : b, backwards ? start : end);
}
it('keeps the selection layer usable if font metrics are unavailable', async () => {
  const error = vi.spyOn(console, 'error').mockImplementation(vi.fn());
  const font = vi.spyOn(Editor.prototype, 'selectionFont').mockReturnValueOnce(undefined);
  try {
    const { root } = await viewer(doc([paragraph('font failure fallback')]));
    expect(font).toHaveBeenCalled();
    expect(error).toHaveBeenCalled();
    expect(getComputedStyle(defined(root.querySelector('[data-selection-run]'))).visibility).toBe('visible');
    expect(copyAll(root).text).toBe('font failure fallback');
  } finally {
    font.mockRestore();
    error.mockRestore();
  }
});
it('handles a real font resource error without terminating the editor', async () => {
  const { root } = await viewer(doc([paragraph('still usable')]));
  const blocks = defined(editor.published?.snapshot.selectionLayout);
  const error = vi.spyOn(console, 'error').mockImplementation(vi.fn());
  try {
    await loadSelectionFonts(
      editor,
      blocks.map((block) => ({
        ...block,
        runs: block.runs.map((run) => ({ ...run, font: { family: 65_535, weight: 400, key: 'missing-font' } })),
      })),
    );
    expect(error).toHaveBeenCalled();
    expect(editor.terminal).toBe(false);
    expect(copyAll(root).text).toBe('still usable');
  } finally {
    error.mockRestore();
  }
});
it('copies Unicode scalar ranges in either direction without changing engine selection', async () => {
  const { root } = await viewer(doc([paragraph('가나다😀라마바')]));
  const initial = editor.appliedSnapshot.selection;
  const spans = [...root.querySelectorAll('[data-selection-run]')];
  const text = spans.map((span) => span.textContent).join('');
  expect(text).toBe('가나다😀라마바');
  select(root, 0, spans.length - 1, 1, defined(defined(spans.at(-1)).textContent).length, true);
  const mapped = defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)));
  expect(mapped).toBeDefined();
  expect(editor.copySelection(mapped)?.text).toBe('나다😀라마바');
  expect(editor.appliedSnapshot.selection).toEqual(initial);
});
it('keeps soft wraps and offscreen text in one native range while scrolling', async ({ task }) => {
  const started = performance.now();
  const text = '긴 한글 문장과 emoji 😀를 여러 줄에 걸쳐 선택합니다. '.repeat(1200);
  const { root, scrollRoot, timings } = await viewer(doc([paragraph(text)]));
  const spans = [...root.querySelectorAll('[data-selection-run]')];
  select(root, 0, spans.length - 1, 0, defined(defined(spans.at(-1)).textContent).length);
  Object.assign(task.meta, {
    performance: { characters: text.length, runs: spans.length, readyMs: performance.now() - started, ...timings },
  });
  const before = defined(window.getSelection()).anchorNode;
  const native = defined(window.getSelection()).toString();
  expect(native).toBe(text);
  scrollRoot.scrollTop = 1500;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await tick();
  await new Promise((resolve) => requestAnimationFrame(resolve));
  expect(defined(window.getSelection()).anchorNode).toBe(before);
  expect(root.querySelectorAll('[data-selection-run]')).toHaveLength(spans.length);
  const last = defined(spans.at(-1));
  const lastRun = defined(defined(editor.published).snapshot.selectionLayout)[0].runs[Number((last as HTMLElement).dataset.selectionRun)];
  const lastRange = document.createRange();
  lastRange.selectNodeContents(last);
  const pages = resolveCachedPageSpans(editor.pageSizes);
  expect(
    Math.abs(lastRange.getBoundingClientRect().top - root.getBoundingClientRect().top - pages[lastRun.page_idx].top - lastRun.rect.y),
  ).toBeLessThan(0.15);
  const mapped = defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)));
  expect(editor.copySelection(mapped)?.text).toBe(text);
}, 90_000);
it('selects an actual word with browser double click', async () => {
  const { root } = await viewer(doc([paragraph('alpha bravo charlie')]));
  const span = defined(root.querySelector<HTMLElement>('[data-selection-run]'));
  await userEvent.dblClick(page.elementLocator(span), { position: { x: 15, y: 8 } });
  expect(defined(window.getSelection()).toString().trim()).toBe('alpha');
  expect(editor.appliedSnapshot.selection).toBeUndefined();
});
it('preserves native word separation across paragraphs and explicit breaks', async () => {
  const { root } = await viewer(
    doc([
      paragraph('first'),
      entry({ type: 'paragraph' }, [
        entry({ type: 'text', text: 'second' }),
        entry({ type: 'hard_break' }),
        entry({ type: 'text', text: 'line' }),
      ]),
      paragraph('third'),
    ]),
  );
  const range = document.createRange();
  range.selectNodeContents(root);
  defined(window.getSelection()).addRange(range);
  expect(defined(window.getSelection()).toString().trim()).toBe('first\nsecond\nline\nthird');
  const mapped = defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)));
  expect(editor.copySelection(mapped)?.text).toBe('first\nsecond\nline\nthird');
});
it.each(['paragraph', 'page'] as const)('exposes a selectable %s break at the end of its line', async (kind) => {
  const first = paragraph('alpha');
  if (kind === 'page') first.children.push(entry({ type: 'page_break' }));
  const plain = doc([first, paragraph('bravo')]);
  plain.root.node = {
    type: 'root',
    layout_mode: {
      type: 'paginated',
      page_width: 320,
      page_height: 240,
      page_margin_top: 40,
      page_margin_bottom: 40,
      page_margin_left: 20,
      page_margin_right: 20,
    },
  };
  const { root } = await viewer(plain);
  const blocks = defined(editor.published?.snapshot.selectionLayout);
  const initial = editor.appliedSnapshot.selection;
  const block = defined(root.querySelector('[data-selection-block="0"]'));
  const marker = defined([...block.querySelectorAll<HTMLElement>('[data-selection-run]')].find((span) => span.textContent === '\n'));
  const text = defined(marker.firstChild);
  const caret = defined(editor.cursorForPosition({ node: blocks[0].node, offset: 5, affinity: 'upstream' }));
  const rect = document.createRange();
  rect.selectNodeContents(marker);
  expect(Math.abs(rect.getBoundingClientRect().left - root.getBoundingClientRect().left - caret.caret.x)).toBeLessThan(0.2);
  expect(Math.abs(rect.getBoundingClientRect().height - caret.caret.height)).toBeLessThan(0.2);
  for (const backwards of [false, true]) {
    defined(window.getSelection()).setBaseAndExtent(text, backwards ? 1 : 0, text, backwards ? 0 : 1);
    const mapped = defined(readNativeSelection(root, blocks));
    expect(mapped.selection.anchor.node).toEqual(blocks[0].node);
    expect(mapped.selection.anchor.offset).toBe(5);
    expect(mapped.selection.head.node).toEqual(blocks[kind === 'paragraph' ? 1 : 0].node);
    expect(mapped.selection.head.offset).toBe(kind === 'paragraph' ? 0 : 6);
    const payload = defined(editor.copySelection(mapped));
    const slice = internalSlice(payload.html);
    if (kind === 'paragraph') {
      expect(payload.text).toBe('\n');
      expect(slice.content.map((fragment: { node: { type: string } }) => fragment.node.type)).toEqual(['paragraph', 'paragraph']);
    } else {
      expect(slice.content.map((fragment: { node: { type: string } }) => fragment.node.type)).toEqual(['page_break']);
    }
    expect(editor.appliedSnapshot.selection).toEqual(initial);
  }
  expect(copyAll(root).text).toBe('alpha\nbravo');
  expect(defined(window.getSelection()).toString().trim()).toBe('alpha\nbravo');
});
it('keeps consecutive empty paragraphs and a page-break-only paragraph distinct', async () => {
  const { root } = await viewer(
    doc([paragraph('first'), paragraph(''), entry({ type: 'paragraph' }, [entry({ type: 'page_break' })]), paragraph('last')]),
  );
  expect(copyAll(root).text).toBe('first\n\n\nlast');
  expect(defined(window.getSelection()).toString().trim()).toBe('first\n\n\nlast');
  const blocks = defined(editor.published?.snapshot.selectionLayout);
  for (const index of [1, 2]) {
    const marker = defined(root.querySelector(`[data-selection-block="${index}"] [data-selection-run]`));
    expect(marker.textContent).toBe('\n');
    const text = defined(marker.firstChild);
    defined(window.getSelection()).setBaseAndExtent(text, 0, text, 1);
    const mapped = defined(readNativeSelection(root, blocks));
    expect(mapped.selection.anchor.offset).toBe(0);
    expect(mapped.selection.head.node).toEqual(blocks[2].node);
    expect(mapped.selection.head.offset).toBe(index === 1 ? 0 : 1);
    const range = defined(window.getSelection()).getRangeAt(0);
    expect(Math.abs(range.getBoundingClientRect().height - blocks[index].runs[0].rect.height)).toBeLessThan(0.2);
  }
});
function copyAll(root: HTMLElement) {
  const range = document.createRange();
  range.selectNodeContents(root);
  defined(window.getSelection()).removeAllRanges();
  defined(window.getSelection()).addRange(range);
  const clipboardData = new DataTransfer();
  const event = new ClipboardEvent('copy', { clipboardData, cancelable: true, bubbles: true });
  document.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  return { text: defined(event.clipboardData).getData('text/plain'), html: defined(event.clipboardData).getData('text/html') };
}
it('copies ruby as semantic HTML through the native copy event', async () => {
  const base = entry({ type: 'text', text: '東' });
  const rest = entry({ type: 'text', text: '京' });
  base.modifiers = { ruby: { type: 'ruby', text: 'とうきょう' }, bold: { type: 'bold' } } as PlainNodeEntry['modifiers'];
  rest.modifiers = { ruby: { type: 'ruby', text: 'とうきょう' } } as PlainNodeEntry['modifiers'];
  const { root } = await viewer(doc([entry({ type: 'paragraph' }, [base, rest])]));
  const payload = copyAll(root);
  const html = new DOMParser().parseFromString(payload.html, 'text/html');
  expect(html.querySelector(':scope ruby strong')?.textContent).toBe('東');
  expect(html.querySelector(':scope ruby rt')?.textContent).toBe('とうきょう');
  expect(html.querySelectorAll('ruby')).toHaveLength(1);
  expect(html.querySelectorAll('rt')).toHaveLength(1);
  expect(payload.text).toBe('東京');
  expect(defined(window.getSelection()).toString().trim()).toBe('東京');
  expect(editor.appliedSnapshot.selection).toBeUndefined();
});
function internalSlice(html: string) {
  const parsed = new DOMParser().parseFromString(html, 'text/html');
  const bytes = Uint8Array.from(atob(defined(defined(parsed.querySelector<HTMLElement>('[data-slice-v2]')).dataset.sliceV2)), (c) =>
    defined(c.codePointAt(0)),
  );
  return JSON.parse(new TextDecoder().decode(bytes));
}
it('includes folded bodies in every viewer copy format when the range crosses the fold', async () => {
  const { root } = await viewer(
    doc([
      paragraph('before'),
      entry({ type: 'fold' }, [
        entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'title' })]),
        entry({ type: 'fold_content' }, [paragraph('secret')]),
      ]),
      paragraph('after'),
    ]),
  );
  expect(root.textContent).not.toContain('secret');
  const payload = copyAll(root);
  expect(payload.text).toBe('before\ntitle\nsecret\nafter');
  expect(payload.html).toContain('secret');
  expect(JSON.stringify(internalSlice(payload.html))).toContain('secret');
  const spans = root.querySelectorAll('[data-selection-run]');
  select(root, 0, spans.length - 1, 2, 2, true);
  const range = defined(readNativeSelection(root, defined(editor.published?.snapshot.selectionLayout)));
  expect(editor.copySelection(range)?.text).toContain('secret');
  const after = defined(root.querySelector(':scope [data-selection-block="2"] [data-selection-run]')?.firstChild);
  defined(window.getSelection()).setBaseAndExtent(defined(spans[0].firstChild), 0, after, 0);
  expect(editor.copySelection(defined(readNativeSelection(root, defined(editor.published?.snapshot.selectionLayout))))?.text).toContain(
    'secret',
  );
  editor.displayZoom = 0.75;
  editor.commitRenderZoom(0.75);
  await tick();
  const title = defined(root.querySelector<HTMLElement>(':scope [data-selection-block="1"] [data-selection-run]'));
  await userEvent.dblClick(page.elementLocator(title), { position: { x: 15, y: 8 } });
  expect(defined(window.getSelection()).toString().trim()).toBe('title');
  expect(root.textContent).not.toContain('secret');
  const titleRun = defined(editor.published?.snapshot.selectionLayout?.[1].runs[0]);
  const togglePosition = {
    x: (titleRun.rect.x - 8) * editor.displayZoom,
    y: (titleRun.rect.y + titleRun.rect.height / 2) * editor.displayZoom,
  };
  await vi.waitFor(() => expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true));
  await userEvent.click(page.elementLocator(root), { position: { ...togglePosition } });
  await vi.waitFor(() => expect(root.textContent).toContain('secret'));
  expect(copyAll(root).text).toBe('before\ntitle\nsecret\nafter');
  await userEvent.click(page.elementLocator(root), { position: { ...togglePosition } });
  await vi.waitFor(() => expect(root.textContent).not.toContain('secret'));
});
it.each([false, true])(
  'includes collapsed bodies with native select-all but keeps title-only selections partial, backwards=%s',
  async (backwards) => {
    const { root } = await viewer(
      doc([
        entry({ type: 'fold' }, [
          entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'title' })]),
          entry({ type: 'fold_content' }, [paragraph('secret')]),
        ]),
      ]),
    );
    expect(root.textContent).not.toContain('secret');
    // The engine keeps a trailing empty paragraph after the fold.
    expect(copyAll(root).text).toBe('title\nsecret\n');
    const blocks = defined(editor.published?.snapshot.selectionLayout);
    select(root, 0, 0, 1, 4, backwards);
    const part = defined(editor.copySelection(defined(readNativeSelection(root, blocks))));
    expect(part.text).toBe('itl');
    expect(part.html).not.toContain('secret');
    expect(JSON.stringify(internalSlice(part.html))).not.toContain('secret');
    select(root, 0, 0, 0, 5, backwards);
    expect(editor.copySelection(defined(readNativeSelection(root, blocks)))?.text).toBe('title');
    const titleBlock = document.createRange();
    titleBlock.selectNodeContents(defined(root.querySelector('[data-selection-block="0"]')));
    defined(window.getSelection()).removeAllRanges();
    defined(window.getSelection()).addRange(titleBlock);
    expect(editor.copySelection(defined(readNativeSelection(root, blocks)))?.text).toBe('title');
    const trailing = defined(root.querySelector<HTMLElement>('[data-selection-block="1"]'));
    const emptyRun = defined(trailing.querySelector('[data-selection-run]'));
    // Native ranges may end at the empty block or at its span, before the <br>.
    for (const end of [trailing, emptyRun]) {
      const title = defined(root.querySelector('[data-selection-run]')?.firstChild);
      defined(window.getSelection()).setBaseAndExtent(backwards ? end : title, 0, backwards ? title : end, 0);
      const range = defined(readNativeSelection(root, blocks));
      expect(range.selection.head.node).toEqual(blocks[1].node);
      expect(editor.copySelection(range)?.text).toBe('title\nsecret\n');
    }
    await userEvent.keyboard(navigator.platform.startsWith('Mac') ? '{Meta>}a{/Meta}' : '{Control>}a{/Control}');
    const all = defined(readNativeSelection(root, blocks));
    expect(all.selection.head.node).toEqual(blocks[1].node);
    const payload = defined(editor.copySelection(all));
    expect(payload.text).toBe('title\nsecret\n');
    expect(payload.html).toContain('secret');
    expect(JSON.stringify(internalSlice(payload.html))).toContain('secret');
  },
);
it('keeps mixed media in order with external URLs and original internal asset references', async () => {
  const { root } = await viewer(
    doc([
      paragraph('first'),
      entry({ type: 'image', id: 'image1' }),
      entry({ type: 'file', id: 'file1' }),
      entry({ type: 'embed', id: 'embed1' }),
      paragraph('last'),
    ]),
  );
  editor.images.assets.set('image1', {
    id: 'image1',
    url: 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7',
    originalUrl: 'https://typie.net/original-images/test.png',
    width: 1,
    height: 1,
    placeholder: '',
  });
  editor.fileAssets.set('file1', { id: 'file1', url: 'https://typie.net/files/notes.pdf', name: 'notes.pdf', size: '1024' });
  editor.embedAssets.set('embed1', {
    id: 'embed1',
    url: 'https://example.com/article',
    title: 'Article title',
    description: 'Description',
    thumbnailUrl: null,
    html: null,
  });
  await tick();
  const labels = [...root.querySelectorAll<HTMLElement>('[data-selection-label]')];
  expect(labels).toHaveLength(5);
  for (const label of labels) expect(label.getBoundingClientRect().height, label.textContent ?? '').toBeGreaterThan(0);
  const payload = copyAll(root);
  expect(payload.text).toBe(
    'first\n이미지\nnotes.pdf (https://typie.net/files/notes.pdf)\nArticle title (https://example.com/article)\nlast',
  );
  const parsed = new DOMParser().parseFromString(payload.html, 'text/html');
  expect(parsed.querySelector('img')?.getAttribute('src')).toBe('https://typie.net/original-images/test.png');
  expect([...parsed.querySelectorAll(':scope [data-root] > *')].map((el) => el.tagName)).toEqual(['P', 'IMG', 'A', 'A', 'P']);
  const slice = internalSlice(payload.html);
  expect(
    slice.content.map(
      (item: {
        node: {
          type: string;
        };
      }) => item.node.type,
    ),
  ).toEqual(['paragraph', 'image', 'file', 'embed', 'paragraph']);
  expect(slice.content[1].node.id).toBe('image1');
  expect(payload.html).not.toContain('data:image');
  const pasted = await Editor.createFromDoc(doc([paragraph('')]), { width: 360, height: 180, scale_factor: 1 });
  try {
    pasted.updateNow((request) => {
      request.enqueue({ type: 'selection', op: { type: 'set_at', page: 0, x: 20, y: 20 } });
      request.enqueue({ type: 'clipboard', op: { type: 'paste', html: payload.html, text: payload.text } });
    });
    const restored = pasted.documentDomProjection().source.root.children;
    expect(restored.filter((item) => ['image', 'file', 'embed'].includes(item.node.type)).map((item) => item.node)).toEqual([
      expect.objectContaining({ type: 'image', id: 'image1' }),
      { type: 'file', id: 'file1' },
      { type: 'embed', id: 'embed1' },
    ]);
  } finally {
    pasted.destroy();
  }
  const first = defined(defined(root.querySelector('[data-selection-run]')).firstChild);
  const last = defined(defined([...root.querySelectorAll('[data-selection-run]')].at(-1)).firstChild);
  defined(window.getSelection()).setBaseAndExtent(first, 2, last, 2);
  expect(editor.copySelection(defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout))))?.text).toBe(
    payload.text.replace('first', 'rst').replace('last', 'la'),
  );
  const label = defined([...root.querySelectorAll('[data-selection-label]')].find((el) => el.textContent?.trim() === 'notes.pdf'));
  const text = defined([...label.childNodes].find((node) => node.nodeType === Node.TEXT_NODE && node.textContent?.includes('notes.pdf')));
  const start = defined(text.textContent).indexOf('notes.pdf');
  defined(window.getSelection()).setBaseAndExtent(text, start + 1, text, start + 4);
  const mapped = defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)));
  const partial = defined(editor.copySelection(mapped));
  expect(partial.text).toBe('ote');
  expect(JSON.stringify(internalSlice(partial.html))).not.toContain('file1');
});
it('copies a linear range through table cells without adding adjacent cell contents', async () => {
  const cell = (text: string) => entry({ type: 'table_cell', col_width: undefined, background_color: undefined }, [paragraph(text)]);
  const { root } = await viewer(
    doc([
      paragraph('before'),
      entry({ type: 'table' }, [
        entry({ type: 'table_row' }, [cell('aaa'), cell('bbb'), cell('ccc')]),
        entry({ type: 'table_row' }, [cell('ddd'), cell('eee'), cell('fff')]),
      ]),
      paragraph('after'),
    ]),
  );
  const spans = [...root.querySelectorAll('[data-selection-run]')];
  const start = defined(defined(spans.find((el) => el.textContent === 'bbb')).firstChild);
  const end = defined(defined(spans.find((el) => el.textContent === 'eee')).firstChild);
  defined(window.getSelection()).setBaseAndExtent(end, 2, start, 1);
  const payload = defined(
    editor.copySelection(defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)))),
  );
  expect(payload.text).toBe('bb\tccc\nddd\tee');
  const parsed = new DOMParser().parseFromString(payload.html, 'text/html');
  expect([...parsed.querySelectorAll('td')].map((el) => el.textContent)).toEqual(['bb', 'ccc', 'ddd', 'ee']);
  expect(parsed.querySelectorAll('tr')).toHaveLength(2);
  const whole = copyAll(root);
  expect(new DOMParser().parseFromString(whole.html, 'text/html').querySelectorAll('td')).toHaveLength(6);
  for (const span of spans) {
    if (!['aaa', 'bbb', 'ccc', 'ddd', 'eee', 'fff'].includes(defined(span.textContent))) continue;
    const rect = span.getBoundingClientRect();
    const point = defined(document.caretPositionFromPoint(rect.left + 4, rect.top + rect.height / 2));
    expect(span.contains(point.offsetNode), defined(span.textContent)).toBe(true);
  }
});
it('clears the old selection on translation and copies the newly selected translated text', async () => {
  const { createDocumentDomMirror } = await import('./document-dom-mirror');
  const { root } = await viewer(doc([paragraph('original')]));
  select(root, 0, 0, 0, 'original'.length);
  const selection = defined(window.getSelection());
  expect(selection.toString()).toBe('original');
  const mirror = createDocumentDomMirror(editor.documentDomProjection());
  defined(mirror.element.querySelector('[data-typie-text]')).textContent = '번역된 문장';
  editor.setDoc(mirror.project().doc);
  await vi.waitFor(() => expect(root.textContent).toContain('번역된 문장'));
  expect(selection.isCollapsed).toBe(true);
  expect(root.getAttribute('translate')).toBe('no');
  const payload = copyAll(root);
  expect(payload.text).toBe('번역된 문장');
  expect(JSON.stringify(internalSlice(payload.html))).not.toContain('original');
});
it('does not copy a translated document before its layout is displayed', async () => {
  const { root } = await viewer(doc([paragraph('displayed original')]));
  const publication = vi.spyOn(editor, 'acceptPublication').mockReturnValue(false);
  try {
    editor.setDoc(doc([paragraph('unseen translation')]));
    await vi.waitFor(() => expect(editor.appliedRevision).toBeGreaterThan(defined(editor.publishedRevision)));
    expect(root.textContent?.trim()).toBe('displayed original');
    expect(copyAll(root)).toEqual({ text: '', html: '' });
  } finally {
    publication.mockRestore();
  }
});
it.each([-10, 20])('uses resolved letter spacing %i in the native text', async (spacing) => {
  const plain = doc([paragraph('iiiiWWWW')]);
  plain.root.modifiers = { letter_spacing: { type: 'letter_spacing', value: spacing } } as PlainNodeEntry['modifiers'];
  const { root } = await viewer(plain);
  const run = defined(defined(editor.published).snapshot.selectionLayout)[0].runs[0];
  expect(run.letter_spacing).toBeCloseTo((run.font_size * spacing) / 100, 5);
  expect(Number.parseFloat(getComputedStyle(defined(root.querySelector('[data-selection-run]'))).letterSpacing)).toBeCloseTo(
    run.letter_spacing,
    5,
  );
});
it('disables viewer selection and copy for protected content and mounts no editor input', async () => {
  await viewer(doc([paragraph('protected')]));
  expect(document.querySelector('[data-selection-handle]')).toBeNull();
  expect(editor.inputEl).toBeUndefined();
  editor.protectContent = true;
  await tick();
  expect(document.querySelector('[data-native-selection-layer]')).toBeNull();
  expect(editor.appliedSnapshot.selectionLayout).toBeUndefined();
  expect(editor.copySelection()).toBeUndefined();
  await vi.waitFor(() => expect(document.querySelector('[data-surface-layer="background"]')).toBeNull());
  editor.protectContent = false;
  await vi.waitFor(() => expect(document.querySelector('[data-native-selection-layer].fonts-ready')).not.toBeNull());
});
it('preserves logical text for bidirectional runs', async () => {
  const text = '시작 hello שלום עולם العربية';
  const { root } = await viewer(doc([paragraph(text)]));
  expect([...root.querySelectorAll('[data-selection-run]')].map((el) => el.textContent).join('')).toBe(text);
  expect(copyAll(root).text).toBe(text);
});
it('keeps document order through a table split across pages without duplicate text', async () => {
  const plain = doc([
    entry({ type: 'table' }, [
      entry({ type: 'table_row' }, [
        entry({ type: 'table_cell', col_width: undefined, background_color: undefined }, [paragraph('left '.repeat(80))]),
        entry({ type: 'table_cell', col_width: undefined, background_color: undefined }, [paragraph('right '.repeat(80))]),
      ]),
    ]),
    paragraph('after'),
  ]);
  plain.root.node = {
    type: 'root',
    layout_mode: {
      type: 'paginated',
      page_width: 320,
      page_height: 240,
      page_margin_left: 20,
      page_margin_right: 20,
      page_margin_top: 20,
      page_margin_bottom: 20,
    },
  };
  const { root } = await viewer(plain);
  expect(editor.pageSizes.length).toBeGreaterThan(2);
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  expect(blocks.map((block) => block.runs.map((run) => run.text).join(''))).toEqual(['left '.repeat(80), 'right '.repeat(80), 'after']);
  expect(copyAll(root).text).toBe(`${'left '.repeat(80)}\t${'right '.repeat(80)}\nafter`);
});
it('corrects native endpoints without replacing nodes or changing the editor selection', async () => {
  const { root, scrollRoot } = await viewer(doc([paragraph('한글과 English 단어, punctuation!? 👩‍💻 emoji를 함께 선택합니다. '.repeat(15))]));
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  const spans = [...root.querySelectorAll<HTMLElement>('[data-selection-run]')];
  const span = defined(spans.find((span) => defined(span.textContent).length > 12));
  const run = blocks[0].runs[Number(span.dataset.selectionRun)];
  const text = defined(span.firstChild);
  const initialSelection = editor.appliedSnapshot.selection;
  const mutations: MutationRecord[] = [];
  const observer = new MutationObserver((records) => {
    mutations.push(...records);
  });
  observer.observe(root, { childList: true, characterData: true, subtree: true });
  try {
    for (const [start, end, backwards] of [
      [1, 5, false],
      [1, 10, false],
      [2, 8, true],
    ] as const) {
      defined(window.getSelection()).setBaseAndExtent(text, backwards ? end : start, text, backwards ? start : end);
      await vi.waitFor(() => {
        for (const offset of [start, end]) {
          const position = {
            node: blocks[0].node,
            offset: run.offset + [...String(run.text.slice(0, offset))].length,
            affinity: 'downstream' as const,
          };
          const caret = defined(editor.cursorForPosition(position));
          const range = document.createRange();
          range.setStart(text, offset);
          range.collapse(true);
          const actual = range.getBoundingClientRect().x - root.getBoundingClientRect().x;
          expect(Math.abs(actual - caret.caret.x)).toBeLessThan(0.1);
        }
      });
      expect(defined(window.getSelection()).toString()).toBe(run.text.slice(start, end));
      expect(span.firstChild).toBe(text);
    }
    scrollRoot.scrollTop = 100;
    await tick();
    expect(defined(window.getSelection()).anchorNode).toBe(text);
    expect(editor.appliedSnapshot.selection).toEqual(initialSelection);
    expect(mutations).toHaveLength(0);
  } finally {
    observer.disconnect();
  }
});

it('keeps native hit testing on the same line in the space after its last run', async () => {
  const { root } = await viewer(
    doc([
      entry({ type: 'paragraph' }, [
        entry({ type: 'text', text: 'first line' }),
        entry({ type: 'hard_break' }),
        entry({ type: 'text', text: 'last line' }),
      ]),
    ]),
  );
  const last = defined([...root.querySelectorAll<HTMLElement>('[data-selection-run]')].find((span) => span.textContent === 'last line'));
  const rect = last.getBoundingClientRect();
  const point = document.caretPositionFromPoint(rect.right + 30, (rect.top + rect.bottom) / 2);
  const prefix = document.createRange();
  prefix.setStart(defined(root.querySelector('[data-selection-run]')?.firstChild), 0);
  prefix.setEnd(defined(point).offsetNode, defined(point).offset);
  expect(prefix.toString()).toBe('first line\nlast line');
});

it('fits every run including emoji before selection and when an endpoint becomes an intermediate run', async () => {
  const flag = entry({ type: 'text', text: '🇯🇵' });
  flag.modifiers = { bold: { type: 'bold' } } as PlainNodeEntry['modifiers'];
  const { root } = await viewer(
    doc([
      entry({ type: 'paragraph' }, [entry({ type: 'text', text: '日本語' }), flag, entry({ type: 'text', text: '日本' })]),
      paragraph('にほにほ。日本にほん'),
    ]),
  );
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  const first = defined(root.querySelector('[data-selection-block="0"]'));
  const spans = [...first.querySelectorAll<HTMLElement>('[data-selection-run]')];
  const last = defined(spans.at(-1));
  const check = () => {
    const origin = root.getBoundingClientRect();
    for (const span of spans) {
      const run = blocks[0].runs[Number(span.dataset.selectionRun)];
      const range = document.createRange();
      range.selectNodeContents(span);
      const actual = range.getBoundingClientRect();
      expect(Math.abs(actual.left - origin.left - run.rect.x), run.text).toBeLessThan(0.15);
      // A newline has no glyph advance; its 1px box is for native hit testing.
      if (run.text !== '\n') expect(Math.abs(actual.width - run.rect.width), run.text).toBeLessThan(0.15);
      expect(Math.abs(actual.height - run.rect.height), run.text).toBeLessThan(0.15);
    }
  };
  check();
  select(root, 0, spans.length - 1, 0, defined(last.textContent).length);
  document.dispatchEvent(new Event('selectionchange'));
  check();
  select(root, 0, spans.length, 0, 2);
  document.dispatchEvent(new Event('selectionchange'));
  check();
});

it('uses the engine caret height for empty paragraphs instead of the line spacing', async () => {
  const { root } = await viewer(doc([paragraph('before'), paragraph(''), paragraph(''), paragraph('after')]));
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  for (const index of [1, 2]) {
    const block = blocks[index];
    const run = block.runs[0];
    const caret = defined(editor.cursorForPosition({ node: block.node, offset: 0, affinity: 'downstream' }));
    expect(run.rect.height).toBeCloseTo(caret.caret.height, 2);
    const span = defined(root.querySelector<HTMLElement>(`[data-selection-block="${index}"] [data-selection-run]`));
    const range = document.createRange();
    range.selectNodeContents(span);
    // Firefox exposes no Range rectangle for an empty <br>; its line box
    // supplies the native paragraph boundary instead.
    const rect = range.getBoundingClientRect();
    const actual = rect.height > 0 ? rect : span.getBoundingClientRect();
    expect(Math.abs(actual.height - caret.caret.height)).toBeLessThan(0.15);
    expect(Math.abs(actual.top - root.getBoundingClientRect().top - caret.caret.y)).toBeLessThan(0.15);
  }
});

it('uses fonts with no visible outlines even when a browser highlight forces an opaque text color', async () => {
  const { root } = await viewer(doc([paragraph('한글 alpha 日本語 🇯🇵 שלום العربية end')]));
  const canvas = document.createElement('canvas');
  canvas.width = 1024;
  canvas.height = 100;
  const context = defined(canvas.getContext('2d'));
  context.fillStyle = 'black';
  for (const span of root.querySelectorAll<HTMLElement>('[data-selection-run]')) {
    context.clearRect(0, 0, canvas.width, canvas.height);
    const style = getComputedStyle(span);
    context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
    context.fillText(defined(span.textContent), 0, 50);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    expect(
      pixels.some((value, index) => index % 4 === 3 && value !== 0),
      `${span.textContent}: ${context.font}`,
    ).toBe(false);
  }
  expect(document.querySelector('[data-surface-layer="background"]')).not.toBeNull();
  expect(document.querySelector('[data-surface-layer="foreground"]')).not.toBeNull();
});

it('keeps page-margin hit testing at the nearest text boundary on that page', async () => {
  const plain = doc([
    entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'first page' }), entry({ type: 'page_break' })]),
    paragraph('second page first paragraph'),
    entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'second page last paragraph' }), entry({ type: 'page_break' })]),
    paragraph('third page'),
  ]);
  plain.root.node = {
    type: 'root',
    layout_mode: {
      type: 'paginated',
      page_width: 320,
      page_height: 240,
      page_margin_top: 40,
      page_margin_bottom: 40,
      page_margin_left: 20,
      page_margin_right: 20,
    },
  };
  const { root, scrollRoot } = await viewer(plain);
  const hit = (pageIndex: number, x: number, y: number) => {
    const rect = defined(editor.pageEls[pageIndex]).getBoundingClientRect();
    const point = defined(document.caretPositionFromPoint(rect.left + x, rect.top + y));
    const element = point.offsetNode instanceof Element ? point.offsetNode : point.offsetNode.parentElement;
    const block = defined(element?.closest<HTMLElement>('[data-selection-block]'));
    const range = document.createRange();
    range.setStart(defined(block.querySelector('[data-selection-run]')?.firstChild), 0);
    range.setEnd(point.offsetNode, point.offset);
    return { block: block.dataset.selectionBlock, text: range.toString() };
  };
  scrollRoot.scrollTop = 140;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await new Promise((resolve) => requestAnimationFrame(resolve));
  for (const x of [5, 100, 315]) expect(hit(0, x, 210)).toEqual({ block: '0', text: 'first page\n' });
  scrollRoot.scrollTop = 240;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await new Promise((resolve) => requestAnimationFrame(resolve));
  for (const x of [5, 100, 315]) expect(hit(1, x, 15)).toEqual({ block: '1', text: '' });
  scrollRoot.scrollTop = 400;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await new Promise((resolve) => requestAnimationFrame(resolve));
  for (const x of [5, 100, 315]) expect(hit(1, x, 210)).toEqual({ block: '2', text: 'second page last paragraph\n' });
  expect(root.querySelectorAll('[data-selection-block]')).toHaveLength(4);
});

it('includes an image when the browser range ends immediately after its image node', async () => {
  const { root } = await viewer(doc([paragraph('before'), entry({ type: 'image', id: 'image1' }), paragraph('after')]));
  editor.images.assets.set('image1', {
    id: 'image1',
    url: 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7',
    originalUrl: 'https://typie.net/original-images/test.png',
    width: 30,
    height: 30,
    placeholder: '',
  });
  await tick();
  const img = defined(root.querySelector('img'));
  const range = document.createRange();
  range.setStart(defined(root.querySelector('[data-selection-run]')?.firstChild), 0);
  range.setEndAfter(img);
  defined(window.getSelection()).addRange(range);
  const mapped = defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)));
  const payload = defined(editor.copySelection(mapped));
  expect(payload.text).toBe('before\n이미지');
  expect(new DOMParser().parseFromString(payload.html, 'text/html').querySelector('img')).not.toBeNull();
});
it('copies only selected visible embed text at either endpoint or within the embed', async () => {
  const { root } = await viewer(doc([paragraph('before'), entry({ type: 'embed', id: 'embed1' }), paragraph('after')]));
  editor.embedAssets.set('embed1', {
    id: 'embed1',
    url: 'https://example.com/article',
    title: 'Article',
    description: null,
    thumbnailUrl: null,
    html: '<div style="height:100px"><span>embedded <strong>title</strong></span><br><span>second line</span><span hidden>hidden</span><style>.unused { color: red }</style><script type="application/json">{"private":"metadata"}</script></div>',
  });
  await tick();
  const content = defined(root.querySelector<HTMLElement>('[data-embed-html]'));
  await vi.waitFor(() => expect(getComputedStyle(content).visibility).toBe('visible'));
  await vi.waitFor(() => expect(editor.isPublished(editor.appliedRevision)).toBe(true));
  const label = defined(content.querySelector('span')?.firstChild);
  const title = defined(content.querySelector('strong')?.firstChild);
  expect(Number.parseFloat(getComputedStyle(defined(title.parentElement)).fontSize)).toBeGreaterThan(0);
  const first = defined(root.querySelector('[data-selection-run]')?.firstChild);
  const last = defined([...root.querySelectorAll('[data-selection-run]')].find((span) => span.textContent === 'after')?.firstChild);
  const selection = defined(window.getSelection());
  const copy = () => defined(editor.copySelection(defined(readNativeSelection(root, defined(editor.published?.snapshot.selectionLayout)))));

  // A backwards range starts inside live HTML and includes the following paragraph.
  selection.setBaseAndExtent(last, 2, label, 3);
  expect(selection.toString()).toContain('edded title');
  const partial = copy();
  expect(partial.text).toBe('edded title\nsecond line\naf');
  expect(JSON.stringify(internalSlice(partial.html))).not.toContain('embed1');
  expect(partial.html).not.toContain('metadata');
  expect(new DOMParser().parseFromString(partial.html, 'text/html').querySelector('script, style, iframe')).toBeNull();

  selection.setBaseAndExtent(first, 0, title, 3);
  expect(copy().text).toBe('before\nembedded tit');
  selection.setBaseAndExtent(title, 1, title, 4);
  expect(copy().text).toBe('itl');
  expect(copyAll(root).html).toContain('https://example.com/article');
});
it('preserves an embed origin when it is the partial selection endpoint', async () => {
  const { root } = await viewer(doc([paragraph('before'), entry({ type: 'embed', id: 'embed1' }), paragraph('after')]));
  editor.embedAssets.set('embed1', {
    id: 'embed1',
    url: 'https://example.com/article',
    title: 'Article',
    description: null,
    thumbnailUrl: null,
    html: null,
  });
  await tick();
  const origin = defined([...root.querySelectorAll('p')].find((e) => e.textContent?.trim() === 'https://example.com'));
  const text = defined([...origin.childNodes].find((e) => e.nodeType === Node.TEXT_NODE));
  const first = defined(root.querySelector('[data-selection-run]')?.firstChild);
  defined(window.getSelection()).setBaseAndExtent(first, 0, text, defined(text.textContent).indexOf('https://example.com') + 8);
  const payload = defined(
    editor.copySelection(defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)))),
  );
  expect(payload.text.endsWith('https://')).toBe(true);
});
it('a paragraph split across pages stops at that page in its bottom margin', async () => {
  const text = 'one two three four five six seven eight nine ten '.repeat(12);
  const plain = doc([paragraph(text)]);
  plain.root.node = {
    type: 'root',
    layout_mode: {
      type: 'paginated',
      page_width: 320,
      page_height: 240,
      page_margin_top: 40,
      page_margin_bottom: 40,
      page_margin_left: 20,
      page_margin_right: 20,
    },
  };
  const { root, scrollRoot } = await viewer(plain);
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  const last = defined(blocks[0].runs.findLast((r) => r.page_idx === 0));
  const expected = last.offset + [...last.text].length;
  scrollRoot.scrollTop = 140;
  scrollRoot.dispatchEvent(new Event('scroll'));
  await new Promise((resolve) => requestAnimationFrame(resolve));
  const allSpans = root.querySelectorAll('[data-selection-run]');
  select(root, 0, allSpans.length - 1, 0, defined(defined(allSpans.item(allSpans.length - 1)).textContent).length);
  expect(defined(window.getSelection()).toString()).toBe(text);
  const pageRect = defined(editor.pageEls[0]).getBoundingClientRect();
  for (const x of [5, 100, 315]) {
    const point = defined(document.caretPositionFromPoint(pageRect.left + x, pageRect.top + 210));
    const start = defined(root.querySelector('[data-selection-run]')?.firstChild);
    defined(window.getSelection()).setBaseAndExtent(start, 0, point.offsetNode, point.offset);
    const mapped = defined(readNativeSelection(root, blocks));
    expect(mapped.selection.head.offset, `x=${x}`).toBe(expected);
  }
});

it('places a hard-break selection rectangle at the preceding text end', async () => {
  const { root } = await viewer(
    doc([
      entry({ type: 'paragraph' }, [
        entry({ type: 'text', text: 'before break' }),
        entry({ type: 'hard_break' }),
        entry({ type: 'text', text: 'after break' }),
      ]),
    ]),
  );
  const blocks = defined(defined(editor.published).snapshot.selectionLayout);
  const runIndex = blocks[0].runs.findIndex((run) => run.text === '\n');
  const run = blocks[0].runs[runIndex];
  const caret = defined(editor.cursorForPosition({ node: blocks[0].node, offset: run.offset, affinity: 'upstream' }));
  expect(run.rect.x).toBeCloseTo(caret.caret.x, 2);
  const span = defined(root.querySelector(`[data-selection-run="${runIndex}"]`));
  const range = document.createRange();
  range.selectNodeContents(span);
  const actual = range.getBoundingClientRect();
  expect(Math.abs(actual.left - root.getBoundingClientRect().left - caret.caret.x)).toBeLessThan(0.2);
  expect(copyAll(root).text).toBe('before break\nafter break');
});

it('preserves fold, link and media controls when protected selection DOM is omitted', async () => {
  const link = entry({ type: 'text', text: 'link' });
  link.modifiers = { link: { type: 'link', href: 'https://example.com/' } } as never;
  const { scrollRoot } = await viewer(
    doc([
      entry({ type: 'paragraph' }, [link]),
      entry({ type: 'fold' }, [
        entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'title' })]),
        entry({ type: 'fold_content' }, [paragraph('hidden')]),
      ]),
      entry({ type: 'file', id: 'file1' }),
      entry({ type: 'embed', id: 'embed1' }),
      entry({ type: 'image', id: 'image1' }),
    ]),
  );
  const nativeLink = defined(document.querySelector<HTMLAnchorElement>('[data-selection-run][href="https://example.com/"]'));
  expect(nativeLink.target).toBe('_blank');
  editor.fileAssets.set('file1', { id: 'file1', url: 'https://typie.net/files/test.pdf', name: 'test.pdf', size: '100' });
  const titleRun = defined(
    editor.published?.snapshot.selectionLayout?.find((block) => block.runs.some((run) => run.text === 'title'))?.runs[0],
  );
  editor.protectContent = true;
  await tick();
  expect(document.querySelector('[data-native-selection-layer]')).toBeNull();
  expect(document.querySelector('a[href="https://example.com/"]')).not.toBeNull();
  expect(defined(document.querySelector<HTMLAnchorElement>('a[href="https://example.com/"]')).target).toBe('_blank');
  expect(document.querySelector('[data-external-element]')).not.toBeNull();
  editor.displayZoom = 1.5;
  editor.commitRenderZoom(1.5);
  await tick();
  await vi.waitFor(() => {
    expect(document.querySelector('[data-surface-layer="background"]')).toBeNull();
    const canvas = defined(editor.pageEls[0]?.querySelector('canvas'));
    const rasterScale = new DOMMatrix(getComputedStyle(canvas).transform).a;
    expect(canvas.getBoundingClientRect().width).toBeCloseTo(Number.parseFloat(canvas.style.width) * rasterScale * 1.5, 1);
    const anchor = defined(document.querySelector<HTMLAnchorElement>('a[href="https://example.com/"]'));
    expect(anchor.getBoundingClientRect().width).toBeCloseTo(Number.parseFloat(anchor.style.width) * 1.5, 1);
  });
  // The narrow harness clips enlarged page margins. Click within the visible
  // scroll viewport so Playwright does not center the entire tall page first.
  editor.displayZoom = 0.75;
  editor.commitRenderZoom(0.75);
  await tick();
  await vi.waitFor(() => expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true));
  const pageRect = defined(editor.pageEls[titleRun.page_idx]).getBoundingClientRect();
  const viewport = scrollRoot.getBoundingClientRect();
  await userEvent.click(page.elementLocator(scrollRoot), {
    position: {
      x: pageRect.left - viewport.left + (titleRun.rect.x - 8) * editor.displayZoom,
      y: pageRect.top - viewport.top + (titleRun.rect.y + titleRun.rect.height / 2) * editor.displayZoom,
    },
  });
  editor.protectContent = false;
  await vi.waitFor(() => expect(document.querySelector('[data-native-selection-layer]')?.textContent).toContain('hidden'));
});

it('virtualizes image pixels without losing image selection boundaries or clipboard assets', async () => {
  const plain = doc(
    Array.from({ length: 8 }, (_, i) => [
      paragraph(`image page ${i}`),
      entry({ type: 'image', id: `image${i}`, proportion: 25 }),
      entry({ type: 'paragraph' }, [entry({ type: 'page_break' })]),
    ]).flat(),
  );
  plain.root.node = {
    type: 'root',
    layout_mode: {
      type: 'paginated',
      page_width: 320,
      page_height: 240,
      page_margin_top: 20,
      page_margin_bottom: 20,
      page_margin_left: 20,
      page_margin_right: 20,
    },
  };
  const { root, scrollRoot } = await viewer(plain);
  const placeholder = '1fsrB38I9wiIh4hwj3CI+AiIgIAICIgA';
  const url = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
  for (let i = 0; i < 8; i++)
    editor.images.assets.set(`image${i}`, {
      id: `image${i}`,
      url: `${url}#${i}`,
      originalUrl: `https://example.com/image${i}.png`,
      width: 100,
      height: 100,
      placeholder,
    });
  await tick();
  const loaded = () =>
    [...root.querySelectorAll<HTMLImageElement>(':scope [data-external-element] img')].filter((img) => img.src.startsWith(url));
  await vi.waitFor(() => expect(loaded().length).toBeGreaterThan(0));
  const first = defined(loaded()[0]);
  const atom = defined(root.querySelector('[data-selection-atom]'));
  const range = document.createRange();
  range.setStart(defined(root.querySelector('[data-selection-run]')?.firstChild), 0);
  range.setEndAfter(atom);
  defined(window.getSelection()).setBaseAndExtent(range.endContainer, range.endOffset, range.startContainer, range.startOffset);
  const endpoints = { start: range.startContainer, startOffset: range.startOffset, end: range.endContainer, endOffset: range.endOffset };
  const selectedPayload = () =>
    defined(editor.copySelection(defined(readNativeSelection(root, defined(defined(editor.published).snapshot.selectionLayout)))));
  const before = selectedPayload();
  expect(new DOMParser().parseFromString(before.html, 'text/html').querySelectorAll('img')).toHaveLength(1);
  const style = getComputedStyle(defined(first.closest('[data-external-element]')));
  expect(style.userSelect ?? style.getPropertyValue('-webkit-user-select')).toBe('none');
  for (let page = 0; page < editor.pageSizes.length; page++) {
    scrollRoot.scrollTop = page * 260;
    scrollRoot.dispatchEvent(new Event('scroll'));
    await new Promise((resolve) => setTimeout(resolve, 40));
  }
  await vi.waitFor(() => expect(first.isConnected).toBe(false));
  expect(loaded().length).toBeLessThan(8);
  expect(root.querySelectorAll('[data-selection-atom]')).toHaveLength(8);
  expect(atom.isConnected).toBe(true);
  const current = defined(window.getSelection()).getRangeAt(0);
  expect(current.startContainer).toBe(endpoints.start);
  expect(current.startOffset).toBe(endpoints.startOffset);
  expect(current.endContainer).toBe(endpoints.end);
  expect(current.endOffset).toBe(endpoints.endOffset);
  expect(selectedPayload()).toEqual(before);
  const all = copyAll(root);
  const html = new DOMParser().parseFromString(all.html, 'text/html');
  expect([...html.querySelectorAll('img')].map((image) => image.src)).toEqual(
    Array.from({ length: 8 }, (_, i) => `https://example.com/image${i}.png`),
  );
  expect(internalSlice(all.html).content.filter((fragment: { node: { type: string } }) => fragment.node.type === 'image')).toHaveLength(8);
}, 30_000);
