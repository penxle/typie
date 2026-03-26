import type { EditorEngine } from '@typie/editor';

let instance: EditorEngine | undefined;
let initPromise: Promise<EditorEngine> | undefined;
let panicked = $state(false);

function wrapWithCrashDetection<T extends object>(target: T): T {
  return new Proxy(target, {
    get(obj, prop) {
      const value = Reflect.get(obj, prop);
      if (typeof value === 'function') {
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
      }
      return value;
    },
  });
}

export function initWasm(): Promise<EditorEngine> {
  return (initPromise ??= import('@typie/editor').then(async ({ default: init, EditorEngine }) => {
    await init();
    instance = wrapWithCrashDetection(new EditorEngine());
    await instance.initGpu();
    return instance;
  }));
}

export const wasm: EditorEngine & { readonly panicked: boolean } = new Proxy({} as EditorEngine & { readonly panicked: boolean }, {
  get(_, prop) {
    if (prop === 'panicked') {
      return panicked;
    }
    if (!instance) {
      throw new Error('WASM not initialized. Call initWasm() first.');
    }
    return Reflect.get(instance, prop);
  },
});
