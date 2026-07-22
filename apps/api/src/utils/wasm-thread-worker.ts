import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { parentPort } from 'node:worker_threads';
import { createInstance } from '@typie/editor-ffi/server';
import type { EditorHost } from '@typie/editor-ffi/server';

if (!parentPort) {
  throw new Error('wasm-thread-worker must run inside a worker thread');
}
const port = parentPort;

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const wasmPath = new URL(import.meta.resolve!('@typie/editor-ffi/server/wasm'));
const wasmModule = await WebAssembly.compile(readFileSync(wasmPath));
const { EditorHost: EditorHostCtor, EditorServer } = await createInstance(wasmModule);
const server = EditorServer.create();

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const icuDataPath = fileURLToPath(import.meta.resolve!('@typie/editor-ffi/server/icu.zst'));
let editorHost: EditorHost | null = null;
const getEditorHost = (): EditorHost => (editorHost ??= EditorHostCtor.create(new Uint8Array(readFileSync(icuDataPath))));

const PROSE_VIEWPORT = { width: 800, height: 1000, scale_factor: 1 };

const extractProse = (editorHost: EditorHost, graph: Uint8Array): string | null => {
  let editor;
  try {
    editor = editorHost.create_editor_from_graph(graph, PROSE_VIEWPORT);
  } catch (err) {
    if (String(err).toLowerCase().includes('no initial cursor position')) return null;
    throw err;
  }
  try {
    return editor.prose_text_annotated();
  } finally {
    editor.free();
  }
};

type Req = { id: number; method: 'collect_fold' | 'consolidate' | 'extract_prose'; args: Uint8Array[] };

port.on('message', ({ id, method, args }: Req) => {
  const startedAt = performance.now();
  try {
    let result: unknown;
    if (method === 'collect_fold') {
      result = server.collect_fold(args[0], args[1]);
    } else if (method === 'consolidate') {
      result = server.consolidate(args[0]);
    } else {
      result = extractProse(getEditorHost(), args[0]);
    }
    port.postMessage({ id, ok: true, result, execMs: performance.now() - startedAt });
  } catch (err) {
    port.postMessage({
      id,
      ok: false,
      poisoned: err instanceof WebAssembly.RuntimeError,
      error: { name: (err as Error).name, message: (err as Error).message, stack: (err as Error).stack },
    });
  }
});

port.postMessage({ id: -1, ok: true });
