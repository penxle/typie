import { afterEach, describe, expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { Editor } from './editor.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry, StableSelection } from '@typie/editor-ffi/browser';

const { captureException } = vi.hoisted(() => ({ captureException: vi.fn() }));

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@sentry/sveltekit', () => ({ captureException }));

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
  node,
  modifiers: {} as never,
  carry: [],
  children,
});

const doc: PlainDoc = {
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 320 } }, [
    entry({ type: 'paragraph' }, [entry({ type: 'text', text: 'hello' })]),
  ]),
};

describe('frozen comment registration', () => {
  let editor: Editor | undefined;

  afterEach(() => {
    editor?.destroy();
    editor = undefined;
    captureException.mockReset();
  });

  it('rejects a malformed selection without failing the editor or blocking valid siblings', async () => {
    await initWasm();
    editor = await Editor.createFromDoc(doc, { width: 320, height: 180, scale_factor: 1 });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    const selection = editor.appliedSnapshot.selection;
    if (!selection) throw new Error('Expected the initial editor selection');
    const valid = editor.freezeSelection(selection);
    if (!valid) throw new Error('Expected a stable initial selection');
    const malformed = { ...valid, version: 'invalid' } as unknown as StableSelection;

    expect(() => editor?.addFrozenComment('malformed', malformed)).not.toThrow();
    editor.addFrozenComment('valid', valid);

    expect(editor.terminal).toBe(false);
    expect(editor.registeredCommentIds()).toEqual(['valid']);
    expect(captureException).toHaveBeenCalledOnce();
  });
});
