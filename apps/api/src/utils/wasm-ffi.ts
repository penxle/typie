import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createInstance } from '@typie/editor-ffi/server';
import type { EditorServer } from '@typie/editor-ffi/server';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const WASM_PATH = fileURLToPath(import.meta.resolve!('@typie/editor-ffi/server/wasm'));
//// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
// const ICU_DATA_PATH = fileURLToPath(import.meta.resolve!('@typie/editor-ffi/server/icu.zst'));

const POOL_SIZE = 10;

// let icuDataPromise: Promise<Uint8Array> | null = null;
// function getIcuData(): Promise<Uint8Array> {
//   return (icuDataPromise ??= readFile(ICU_DATA_PATH).then((buf) => new Uint8Array(buf)));
// }

async function createHost(module: WebAssembly.Module): Promise<EditorServer> {
  const { EditorServer } = await createInstance(module);
  return EditorServer.create();
}

const available: EditorServer[] = [];
const waiting: ((host: EditorServer) => void)[] = [];
let poolReady: Promise<void> | null = null;
let wasmModule: WebAssembly.Module | null = null;

async function initPool(): Promise<void> {
  const wasmBuffer = await readFile(WASM_PATH);
  wasmModule = await WebAssembly.compile(wasmBuffer);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const hosts = await Promise.all(Array.from({ length: POOL_SIZE }, () => createHost(wasmModule!)));
  available.push(...hosts);
}

function returnToPool(host: EditorServer): void {
  const next = waiting.shift();
  if (next) next(host);
  else available.push(host);
}

async function use<T>(fn: (host: EditorServer) => T): Promise<Awaited<T>> {
  if (!poolReady) {
    poolReady = initPool().catch((err) => {
      poolReady = null;
      throw err;
    });
  }
  await poolReady;

  const host =
    available.pop() ??
    (await new Promise<EditorServer>((resolve) => {
      waiting.push(resolve);
    }));
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

export const wasm: Async<EditorServer> = new Proxy({} as Async<EditorServer>, {
  get: (_, prop: string | symbol) => {
    if (prop === 'use') return use;
    return async (...args: unknown[]) =>
      use((host) => (host as unknown as Record<string | symbol, (...a: unknown[]) => unknown>)[prop](...args));
  },
});
