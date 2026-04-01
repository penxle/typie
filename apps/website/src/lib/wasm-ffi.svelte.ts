import { createInstance } from '@typie/editor-ffi';
import wasmUrl from '@typie/editor-ffi/wasm?url';
import type { EditorHost } from '@typie/editor-ffi';

let host: EditorHost | undefined;
let hostPromise: Promise<EditorHost> | undefined;
const panicked = $state(false);

export function initWasm(): Promise<EditorHost> {
  return (hostPromise ??= (async () => {
    const mod = await WebAssembly.compileStreaming(fetch(wasmUrl));
    const { EditorHost } = await createInstance(mod);
    host = await EditorHost.create('Gpu');
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
