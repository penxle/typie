import { createInstance } from '@typie/editor-ffi/browser';
import icuUrl from '@typie/editor-ffi/browser/icu.zst?url';
import wasmUrl from '@typie/editor-ffi/browser/wasm?url';
import { destroyAll, failAll } from '$lib/editor-ffi/registry';
import { registerWasmHmrCleanup } from '$lib/wasm-hmr';
import type { EditorHost } from '@typie/editor-ffi/browser';

let host: EditorHost | undefined;
let hostPromise: Promise<EditorHost> | undefined;
let failure = $state.raw<WebAssembly.RuntimeError>();
let disposed = false;

export function initWasm(): Promise<EditorHost> {
  if (failure !== undefined) return Promise.reject(failure);
  if (disposed) return Promise.reject(new Error('Editor WASM was disposed for HMR.'));

  return (hostPromise ??= (async () => {
    const [mod, icuData] = await Promise.all([
      WebAssembly.compileStreaming(fetch(wasmUrl)),
      fetch(icuUrl)
        .then((r) => r.arrayBuffer())
        .then((b) => new Uint8Array(b)),
    ]);

    const { EditorHost } = await createInstance(mod, (error) => {
      failure = error;
      failAll(error);
    });
    const createdHost = EditorHost.create(icuData);
    if (disposed) {
      createdHost.free();
      throw new Error('Editor WASM initialization was canceled for HMR.');
    }
    host = createdHost;
    return host;
  })());
}

function disposeWasm(): void {
  if (disposed) return;
  disposed = true;
  destroyAll();
  const currentHost = host;
  host = undefined;
  hostPromise = undefined;
  currentHost?.free();
}

registerWasmHmrCleanup(import.meta.hot, disposeWasm);

export const wasm: EditorHost & { readonly panicked: boolean } = new Proxy({} as EditorHost & { readonly panicked: boolean }, {
  get(_, prop) {
    if (prop === 'panicked') {
      return failure !== undefined;
    }

    if (!host) {
      throw new Error('WASM not initialized. Call initWasm() first.');
    }

    const value = Reflect.get(host, prop);
    return typeof value === 'function' ? value.bind(host) : value;
  },
});
