import '../../app.css';

import { afterEach, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { Editor } from './editor.svelte';
import { clearSelectionDragGhost, setSelectionDragGhost } from './selection-drag-ghost';
import type { DragGhost, PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

let editor: Editor | undefined;
afterEach(() => {
  clearSelectionDragGhost();
  editor?.destroy();
  editor = undefined;
});

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({ node, children, modifiers: {} as never, carry: [] });
const paragraph = (text: string) => entry({ type: 'paragraph' }, [entry({ type: 'text', text })]);
const doc = (children: PlainNodeEntry[]): PlainDoc => ({
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, children),
});
const badge = () => document.querySelector<HTMLElement>('[data-selection-drag-ghost]');

it('fits both excerpt ends and wraps every block group without splitting icon/count pairs', async () => {
  const ghost: DragGhost = {
    kind: 'paragraph',
    text: `${'👩🏽‍💻'.repeat(8)}…${'한'.repeat(6)}`,
    blocks: ['image', 'file', 'embed', 'table', 'callout', 'fold', 'horizontal_rule', 'page_break', 'archived', 'unknown'].map((kind) => ({
      kind,
      count: 999,
    })) as DragGhost['blocks'],
  };
  const transfer = new DataTransfer();
  const capture = vi.spyOn(transfer, 'setDragImage');
  setSelectionDragGhost(transfer, ghost);
  const element = badge();
  if (!element) throw new Error('Expected a mounted ghost');
  expect(element.getBoundingClientRect().width).toBeLessThanOrEqual(320);
  const excerpt = element.querySelector<HTMLElement>('[data-drag-excerpt]');
  if (!excerpt) throw new Error('Expected an excerpt');
  expect(excerpt.textContent).toBe(ghost.text);
  expect(excerpt.scrollWidth).toBeLessThanOrEqual(excerpt.clientWidth + 1);
  const groups = [...element.querySelectorAll<HTMLElement>('[data-drag-kind]')];
  expect(groups).toHaveLength(10);
  expect(new Set(groups.map((group) => group.offsetTop)).size).toBeGreaterThan(1);
  expect(groups.every((group) => group.getBoundingClientRect().height === 20)).toBe(true);
  expect(element.style.borderRadius).toBe('12px');
  expect(getComputedStyle(element).pointerEvents).toBe('none');
  expect(capture).toHaveBeenCalledWith(element, -8, -12);
  expect(getComputedStyle(element).visibility).toBe('visible');
  expect(element.querySelectorAll('svg')).toHaveLength(11);
  await new Promise(requestAnimationFrame);
  expect(badge()).toBeNull();
});

it('fits a full short excerpt with emoji and decomposed Hangul without rewriting it', () => {
  for (const cluster of ['👩🏽‍💻', '한', '한', '🇰🇷']) {
    const text = cluster.repeat(15);
    setSelectionDragGhost(new DataTransfer(), { kind: 'paragraph', text, blocks: [] });
    const excerpt = badge()?.querySelector<HTMLElement>('[data-drag-excerpt]');
    if (!excerpt) throw new Error('Expected an excerpt');
    expect(excerpt.textContent).toBe(text);
    expect(excerpt.scrollWidth).toBeLessThanOrEqual(excerpt.clientWidth + 1);
    expect(badge()?.getBoundingClientRect().height).toBe(28);
  }
});

it('renders icon-only groups without an empty text row or singleton numbers', () => {
  setSelectionDragGhost(new DataTransfer(), {
    text: '',
    kind: 'paragraph',
    blocks: [
      { kind: 'image', count: 3 },
      { kind: 'table', count: 1 },
    ],
  });
  expect(badge()?.querySelector('[data-drag-excerpt]')).toBeNull();
  expect(badge()?.querySelector('[data-drag-kind="image"]')?.textContent?.trim()).toBe('3');
  expect(badge()?.querySelector('[data-drag-kind="table"]')?.textContent?.trim()).toBe('');
  expect(badge()?.getBoundingClientRect().height).toBe(28);
});

it('keeps native drag data and cleans up after a trusted browser drag', async () => {
  const source = document.createElement('div');
  source.draggable = true;
  source.textContent = 'Drag selected content';
  source.style.cssText = 'width:200px;height:40px;margin:40px;transform:scale(1.5)';
  const target = document.createElement('div');
  target.style.cssText = 'width:300px;height:100px;margin:80px';
  target.textContent = 'Drop here';
  const events: string[] = [];
  source.addEventListener('dragstart', (event) => {
    expect(event.isTrusted).toBe(true);
    const transfer = event.dataTransfer;
    if (!transfer) throw new Error('Expected native drag data');
    transfer.setData('text/plain', 'selected text');
    transfer.effectAllowed = 'copyMove';
    setSelectionDragGhost(transfer, { text: 'selected text', kind: 'paragraph', blocks: [] });
    expect(badge()?.getBoundingClientRect().height).toBe(28);
    events.push('start');
  });
  target.addEventListener('dragover', (event) => event.preventDefault());
  target.addEventListener('drop', (event) => {
    event.preventDefault();
    expect(event.dataTransfer?.getData('text/plain')).toBe('selected text');
    events.push('drop');
  });
  source.addEventListener('dragend', () => {
    clearSelectionDragGhost();
    events.push('end');
  });
  document.body.append(source, target);
  try {
    await userEvent.dragAndDrop(source, target);
    expect(events).toEqual(['start', 'drop', 'end']);
    expect(badge()).toBeNull();
  } finally {
    source.remove();
    target.remove();
  }
});

it('uses the same WASM slice for payload and ghost including offscreen/folded content', async () => {
  editor = await Editor.createFromDoc(
    doc([
      paragraph('first'),
      entry({ type: 'image', id: undefined, proportion: 100 }),
      entry({ type: 'fold' }, [
        entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'title' })]),
        entry({ type: 'fold_content' }, [paragraph('hidden')]),
      ]),
      paragraph('last'),
    ]),
    { width: 360, height: 180, scale_factor: 1 },
  );
  editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } }));
  const before = editor.appliedSnapshot.selection;
  const ordinary = editor.copySelection();
  const copied = editor.copySelection({ includeDragGhost: true });
  expect(copied?.html).toBe(ordinary?.html);
  expect(copied?.text).toBe(ordinary?.text);
  expect(copied?.drag_ghost).toEqual({
    text: 'first ti…n last',
    kind: 'paragraph',
    blocks: [
      { kind: 'image', count: 1 },
      { kind: 'fold', count: 1 },
    ],
  });
  expect(ordinary?.drag_ghost).toBeUndefined();
  expect(editor.appliedSnapshot.selection).toEqual(before);
  editor.readOnly = true;
  editor.protectContent = true;
  expect(editor.copySelection({ includeDragGhost: true })).toBeUndefined();
});

it('displays the Rust excerpt verbatim, including an ellipsis from the selected text', async () => {
  editor = await Editor.createFromDoc(doc([paragraph(`${'👩🏽‍💻'.repeat(16)}끝…`)]), {
    width: 360,
    height: 180,
    scale_factor: 1,
  });
  editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } }));
  const ghost = editor.copySelection({ includeDragGhost: true })?.drag_ghost;
  if (!ghost) throw new Error('Expected a drag ghost');
  expect(ghost.text).toBe(`${'👩🏽‍💻'.repeat(8)}…${'👩🏽‍💻'.repeat(4)}끝…`);
  setSelectionDragGhost(new DataTransfer(), ghost);
  const excerpt = badge()?.querySelector<HTMLElement>('[data-drag-excerpt]');
  if (!excerpt) throw new Error('Expected an excerpt');
  expect(excerpt.textContent).toBe(ghost.text);
  expect(excerpt.scrollWidth).toBeLessThanOrEqual(excerpt.clientWidth + 1);
});

it('bounds the ghost for a long selection and reports additional dragstart work', async () => {
  editor = await Editor.createFromDoc(doc([paragraph('start ' + 'long text '.repeat(1000) + 'end')]), {
    width: 360,
    height: 180,
    scale_factor: 1,
  });
  editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } }));
  const samples = [];
  for (const includeDragGhost of [false, true, false, true]) {
    const start = performance.now();
    const copied = editor.copySelection({ includeDragGhost });
    samples.push({ includeDragGhost, ms: performance.now() - start });
    if (includeDragGhost) {
      expect(copied?.drag_ghost?.text).toMatch(/^start .*….*end$/u);
      expect(copied?.drag_ghost?.text.length).toBeLessThan(200);
    }
  }
  console.info('drag ghost copy timings (development WASM)', samples);
}, 30_000);
