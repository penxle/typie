import type { Application } from '@typie/editor';

let instance: Application | undefined;
let initPromise: Promise<Application> | undefined;

export function initWasm(): Promise<Application> {
  return (initPromise ??= import('@typie/editor').then(({ Application }) => {
    instance = new Application();
    return instance;
  }));
}

export const wasm: Application = new Proxy({} as Application, {
  get(_, prop) {
    if (!instance) {
      throw new Error('WASM not initialized. Call initWasm() first.');
    }
    const value = Reflect.get(instance, prop);
    if (typeof value === 'function') {
      return value.bind(instance);
    }
    return value;
  },
});
