import type { Application } from '@typie/editor';

let instance: Application | undefined;
let initPromise: Promise<Application> | undefined;
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

export function initWasm(): Promise<Application> {
  return (initPromise ??= import('@typie/editor').then(({ Application }) => {
    instance = wrapWithCrashDetection(new Application());
    return instance;
  }));
}

export const wasm: Application & { readonly panicked: boolean } = new Proxy({} as Application & { readonly panicked: boolean }, {
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
