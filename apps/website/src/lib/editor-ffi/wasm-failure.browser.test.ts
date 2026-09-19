import { expect, it, vi } from 'vitest';
import { DocumentEditingSession } from '$lib/document-editing/session';
import { documentEditing } from '$lib/document-editing/state.svelte';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { IndexeddbDeltaStore } from '../../routes/website/(dashboard)/[slug]/v2/sync/store';
import { Editor } from './editor.svelte';
import { snapshot } from './registry';
import type { PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));

it('contains an ordinary error locally and stops every editor after a rendering trap without reentering WASM', async () => {
  const trap = new WebAssembly.Instance(
    new WebAssembly.Module(
      new Uint8Array([
        0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 7, 8, 1, 4, 116, 114, 97, 112, 0, 0, 10, 5, 1, 3, 0, 0, 11,
      ]),
    ),
  ).exports.trap as () => void;
  const instantiate = WebAssembly.instantiate.bind(WebAssembly);
  let shouldTrap = false;
  let calls = 0;
  vi.spyOn(WebAssembly, 'instantiate').mockImplementation((async (module: WebAssembly.Module, imports: WebAssembly.Imports) => {
    const instance = await instantiate(module, imports);
    return {
      exports: Object.fromEntries(
        Object.entries(instance.exports).map(([name, value]) => [
          name,
          typeof value === 'function'
            ? (...args: unknown[]) => {
                calls++;
                if (shouldTrap && name === 'editor_render_surface') trap();
                return value(...args);
              }
            : value,
        ]),
      ),
    };
  }) as typeof WebAssembly.instantiate);
  vi.spyOn(console, 'error').mockImplementation(() => false);
  const plain: PlainDoc = {
    root: {
      node: { type: 'root', layout_mode: { type: 'continuous', max_width: 400 } },
      modifiers: {} as PlainNodeEntry['modifiers'],
      carry: [],
      children: [{ node: { type: 'paragraph' }, modifiers: {} as PlainNodeEntry['modifiers'], carry: [], children: [] }],
    },
  };
  const viewport = { width: 400, height: 300, scale_factor: 1 };
  const editors: Editor[] = [];
  const sessions: DocumentEditingSession[] = [];
  try {
    for (let index = 0; index < 3; index++) {
      const editor = await Editor.createFromDoc(plain, viewport);
      editors.push(editor);
      const heads = editor.currentHeads();
      const session = DocumentEditingSession.create({
        documentId: `document-${index}`,
        paneId: `pane-${index}`,
        title: () => `Document ${index}`,
        entity: undefined,
        editor,
        store: new IndexeddbDeltaStore(),
        capturedHeads: heads,
        snapshot: { heads, durableHeads: heads, seq: '' },
        connection: { push: vi.fn(), pull: vi.fn() },
        onReload: vi.fn(),
        onError: vi.fn(),
      });
      sessions.push(session);
      documentEditing.register(session);
    }
    const [ordinary, rendering, sibling] = editors;
    ordinary.fail(new Error('ordinary engine failure'));
    expect(wasm.panicked).toBe(false);
    expect(sibling.currentHeads().length).toBeGreaterThan(0);
    expect(rendering.terminal).toBe(false);
    expect(documentEditing.sessions).toEqual(sessions.slice(1));

    rendering.activateVisualHost();
    shouldTrap = true;
    expect(() => rendering.attachSurface(0, document.createElement('div'), 400, 300)).toThrow(WebAssembly.RuntimeError);
    const callsAfterTrap = calls;
    expect(wasm.panicked).toBe(true);
    expect(rendering.terminal).toBe(true);
    expect(sibling.terminal).toBe(true);
    expect(sibling.failure).toBe(rendering.failure);
    expect(snapshot()).toEqual([]);
    expect(documentEditing.sessions).toEqual([]);
    expect(documentEditing.hasUnprotectedChanges()).toBe(false);
    expect(() => sibling.currentHeads()).toThrow(WebAssembly.RuntimeError);
    const departure = await documentEditing.prepareDeparture(() => documentEditing.sessions, 'reload');
    expect(departure).not.toBe(false);
    if (departure) departure.release();
    expect(sibling.destroyed).toBe(true);
    await expect(initWasm()).rejects.toBe(rendering.failure);
    await expect(Editor.createFromDoc(plain, viewport)).rejects.toBe(rendering.failure);
    for (const editor of editors) editor.destroy();
    expect(calls).toBe(callsAfterTrap);
  } finally {
    for (const session of sessions) session.dispose();
    for (const editor of editors) editor.destroy();
    vi.restoreAllMocks();
  }
});
