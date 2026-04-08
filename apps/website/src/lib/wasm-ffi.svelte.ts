import { createInstance } from '@typie/editor-ffi/browser';
import icuUrl from '@typie/editor-ffi/browser/icu.zst?url';
import wasmUrl from '@typie/editor-ffi/browser/wasm?url';
import type { EditorHost } from '@typie/editor-ffi/browser';

let host: EditorHost | undefined;
let hostPromise: Promise<EditorHost> | undefined;
const panicked = $state(false);

export function initWasm(): Promise<EditorHost> {
  return (hostPromise ??= (async () => {
    const [mod, icuData] = await Promise.all([
      WebAssembly.compileStreaming(fetch(wasmUrl)),
      fetch(icuUrl)
        .then((r) => r.arrayBuffer())
        .then((b) => new Uint8Array(b)),
    ]);

    const { EditorHost } = await createInstance(mod);
    host = await EditorHost.create('gpu', icuData);
    return host;
  })());
}

export const wasm: EditorHost & { readonly panicked: boolean } = new Proxy({} as EditorHost & { readonly panicked: boolean }, {
  get(_, prop) {
    if (prop === 'panicked') {
      return panicked;
    }

    if (!host) {
      throw new Error('WASM not initialized. Call initWasm() first.');
    }

    return Reflect.get(host, prop);
  },
});
