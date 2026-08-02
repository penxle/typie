import { createInstance } from '@typie/editor-ffi/browser';
import icuUrl from '@typie/editor-ffi/browser/icu.zst?url';
import wasmUrl from '@typie/editor-ffi/browser/wasm?url';
import type { EditorHost } from '@typie/editor-ffi/browser';

let host: EditorHost | undefined;
let hostPromise: Promise<EditorHost> | undefined;
let panicked = $state(false);

function wrapWithCrashDetection<T extends object>(target: T): T {
  return new Proxy(target, {
    get(obj, prop) {
      const value = Reflect.get(obj, prop);
      if (typeof value !== 'function') {
        return value;
      }

      return (...args: unknown[]) => {
        try {
          const result = value.apply(obj, args);
          if (result != null && typeof result === 'object' && '__wbg_ptr' in result) {
            return wrapWithCrashDetection(result);
          }

          return result;
        } catch (err) {
          if (err instanceof WebAssembly.RuntimeError) {
            panicked = true;
          }

          throw err;
        }
      };
    },
  });
}

export function initWasm(): Promise<EditorHost> {
  return (hostPromise ??= (async () => {
    const [mod, icuData] = await Promise.all([
      WebAssembly.compileStreaming(fetch(wasmUrl)),
      fetch(icuUrl)
        .then((r) => r.arrayBuffer())
        .then((b) => new Uint8Array(b)),
    ]);

    const { EditorHost } = await createInstance(mod);
    host = wrapWithCrashDetection(EditorHost.create(icuData));
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
