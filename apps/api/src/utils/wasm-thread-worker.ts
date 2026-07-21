import { readFileSync } from 'node:fs';
import { parentPort } from 'node:worker_threads';
import { createInstance } from '@typie/editor-ffi/server';

if (!parentPort) {
  throw new Error('wasm-thread-worker must run inside a worker thread');
}
const port = parentPort;

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const wasmPath = new URL(import.meta.resolve!('@typie/editor-ffi/server/wasm'));
const wasmModule = await WebAssembly.compile(readFileSync(wasmPath));
const { EditorServer } = await createInstance(wasmModule);
const host = EditorServer.create();

type Req = { id: number; method: 'collect_fold' | 'consolidate'; args: Uint8Array[] };

port.on('message', ({ id, method, args }: Req) => {
  const startedAt = performance.now();
  try {
    const result = method === 'collect_fold' ? host.collect_fold(args[0], args[1]) : host.consolidate(args[0]);
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
