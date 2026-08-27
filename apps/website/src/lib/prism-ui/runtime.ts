import { createPrismRuntime } from '@typie/prism-ui';
import { createInstance } from '@typie/prism-ui-web/browser';
import wasmUrl from '@typie/prism-ui-web/browser/wasm?url';
import { registerWasmHmrCleanup } from '$lib/wasm-hmr';

export const prismRuntime = createPrismRuntime({
  loadRenderer: async () => {
    const wasmModule = await WebAssembly.compileStreaming(fetch(wasmUrl));
    const module = await createInstance(wasmModule);
    return {
      PrismWebRuntime: module.PrismWebRuntime,
    };
  },
});

registerWasmHmrCleanup(import.meta.hot, () => prismRuntime.destroy());
