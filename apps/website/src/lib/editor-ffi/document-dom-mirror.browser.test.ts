import { afterEach, describe, expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { createDocumentDomMirror } from './document-dom-mirror';
import { Editor } from './editor.svelte';
import type { Modifier, ModifierType, PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const entry = (
  node: PlainNode,
  children: PlainNodeEntry[] = [],
  modifiers: Partial<Record<ModifierType, Modifier>> = {},
): PlainNodeEntry => ({
  node,
  children,
  modifiers: modifiers as Record<ModifierType, Modifier>,
  carry: [],
});

const source: PlainDoc = {
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 640 } }, [
    entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'Before' }, [], { bold: { type: 'bold' } }), entry({ type: 'page_break' })]),
  ]),
};

describe('document DOM mirror Rust-Web contract', () => {
  let editor: Editor | undefined;

  afterEach(() => {
    editor?.destroy();
    editor = undefined;
  });

  it('projects real editor HTML without duplicating its marker schema in Web fixtures', async () => {
    await initWasm();
    editor = await Editor.createFromDoc(source, { width: 640, height: 480, scale_factor: 1 });

    const mirror = createDocumentDomMirror(editor.documentDomProjection());
    const run = mirror.element.querySelector<HTMLElement>('span');
    if (!run) throw new Error('expected a text carrier from the Rust document projection');
    run.textContent = '이후';

    const [paragraph] = mirror.project().doc.root.children;
    expect(paragraph?.children.map((child) => child.node)).toEqual([{ type: 'text', text: '이후' }, { type: 'page_break' }]);
    expect(paragraph?.children[0]?.modifiers.bold).toEqual({ type: 'bold' });
  });
});
