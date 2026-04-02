import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createInstance } from '@typie/editor-ffi/server';
import type { EditorHost } from '@typie/editor-ffi/server';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const WASM_PATH = fileURLToPath(import.meta.resolve!('@typie/editor-ffi/server/wasm'));

const POOL_SIZE = 10;

async function createHost(module: WebAssembly.Module): Promise<EditorHost> {
  const { EditorHost } = await createInstance(module);
  return EditorHost.create('gpu');
}

const available: EditorHost[] = [];
const waiting: ((host: EditorHost) => void)[] = [];
let poolReady: Promise<void> | null = null;
let wasmModule: WebAssembly.Module | null = null;

async function initPool(): Promise<void> {
  const wasmBuffer = await readFile(WASM_PATH);
  wasmModule = await WebAssembly.compile(wasmBuffer);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const hosts = await Promise.all(Array.from({ length: POOL_SIZE }, () => createHost(wasmModule!)));
  available.push(...hosts);
}

function returnToPool(host: EditorHost): void {
  const next = waiting.shift();
  if (next) next(host);
  else available.push(host);
}

async function use<T>(fn: (host: EditorHost) => T): Promise<Awaited<T>> {
  if (!poolReady) {
    poolReady = initPool().catch((err) => {
      poolReady = null;
      throw err;
    });
  }
  await poolReady;

  const host = available.pop() ?? (await new Promise<EditorHost>((resolve) => waiting.push(resolve)));
  try {
    const result = await fn(host);
    returnToPool(host);
    return result;
  } catch (err) {
    if (err instanceof WebAssembly.RuntimeError && wasmModule) {
      try {
        returnToPool(await createHost(wasmModule));
      } catch {
        /* noop */
      }
    } else {
      returnToPool(host);
    }
    throw err;
  }
}

type Async<T> = {
  [K in keyof T]: T[K] extends (...args: infer A) => infer R ? (...args: A) => Promise<Awaited<R>> : T[K];
} & {
  use<R>(fn: (host: T) => R): Promise<Awaited<R>>;
};

export const wasm: Async<EditorHost> = new Proxy({} as Async<EditorHost>, {
  get: (_, prop: string | symbol) => {
    if (prop === 'use') return use;
    return async (...args: unknown[]) =>
      use((host) => (host as unknown as Record<string | symbol, (...a: unknown[]) => unknown>)[prop](...args));
  },
});
