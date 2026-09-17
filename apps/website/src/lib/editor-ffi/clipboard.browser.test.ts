import { expect, it, vi } from 'vitest';
import { Editor } from './editor.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@mearie/svelte', async (importOriginal) => {
  const original = await importOriginal<typeof import('@mearie/svelte')>();
  return { ...original, createMutation: () => [vi.fn()] };
});

it('includes folded content in every editor copy format without changing the selection', async () => {
  const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
    node,
    children,
    modifiers: {} as never,
    carry: [],
  });
  const paragraph = (text: string) => entry({ type: 'paragraph' }, [entry({ type: 'text', text })]);
  const plain: PlainDoc = {
    root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, [
      paragraph('before'),
      entry({ type: 'fold' }, [
        entry({ type: 'fold_title' }, [entry({ type: 'text', text: 'title' })]),
        entry({ type: 'fold_content' }, [paragraph('secret')]),
      ]),
      paragraph('after'),
    ]),
  };
  const editor = await Editor.createFromDoc(plain, { width: 360, height: 180, scale_factor: 1 });
  try {
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } }));
    const selection = editor.appliedSnapshot.selection;
    expect(selection).toBeDefined();
    for (const payload of [editor.copySelection(), editor.copySelection({ selection })]) {
      expect(payload?.text).toBe('before\ntitle\nsecret\nafter');
      if (!payload) throw new Error('Expected clipboard payload');
      expect(payload.html).toContain('secret');
      const html = new DOMParser().parseFromString(payload.html, 'text/html');
      const encoded = html.querySelector<HTMLElement>('[data-slice-v2]')?.dataset.sliceV2;
      if (!encoded) throw new Error('Expected internal clipboard slice');
      const bytes = Uint8Array.from(atob(encoded), (character) => character.codePointAt(0) ?? 0);
      expect(new TextDecoder().decode(bytes)).toContain('secret');
      expect(editor.appliedSnapshot.selection).toEqual(selection);
    }
  } finally {
    editor.destroy();
  }
});
