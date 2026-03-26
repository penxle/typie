import { readFile } from 'node:fs/promises';
import type { EditorEngine } from '@typie/editor';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const base = import.meta.resolve!('@typie/editor');
const WASM_PATH = new URL('editor_bg.wasm', base).pathname;
const GLUE_PATH = new URL('editor.js', base).pathname;

const POOL_SIZE = 10;

const glueSource = await readFile(GLUE_PATH, 'utf8');
const isolatedSource = glueSource
  .replaceAll(/^export \{[^}]*\}.*$/gm, '')
  .replaceAll(/^export /gm, '')
  .replaceAll('import.meta.url', '""');

type IsolatedScope = {
  initSync: (input: { module: WebAssembly.Module }) => void;
  EditorEngine: new () => EditorEngine;
};

function createInstance(module: WebAssembly.Module): EditorEngine {
  const { initSync, EditorEngine } = new Function(`"use strict";\n${isolatedSource}\nreturn{initSync,EditorEngine};`)() as IsolatedScope;
  initSync({ module });
  return new EditorEngine();
}

const available: EditorEngine[] = [];
const waiting: ((app: EditorEngine) => void)[] = [];
let poolReady: Promise<void> | null = null;
let wasmModule: WebAssembly.Module | null = null;

async function initPool(): Promise<void> {
  const wasmBuffer = await readFile(WASM_PATH);
  wasmModule = await WebAssembly.compile(wasmBuffer);
  for (let i = 0; i < POOL_SIZE; i++) {
    available.push(createInstance(wasmModule));
  }
}

function returnToPool(app: EditorEngine): void {
  const next = waiting.shift();
  if (next) next(app);
  else available.push(app);
}

async function use<T>(fn: (app: EditorEngine) => T): Promise<Awaited<T>> {
  if (!poolReady) {
    poolReady = initPool().catch((err) => {
      poolReady = null;
      throw err;
    });
  }
  await poolReady;

  const app = available.pop() ?? (await new Promise<EditorEngine>((resolve) => waiting.push(resolve)));
  try {
    const result = await fn(app);
    returnToPool(app);
    return result;
  } catch (err) {
    if (err instanceof WebAssembly.RuntimeError && wasmModule) {
      try {
        returnToPool(createInstance(wasmModule));
      } catch {
        /* noop */
      }
    } else {
      returnToPool(app);
    }
    throw err;
  }
}

type Async<T> = {
  [K in keyof T]: T[K] extends (...args: infer A) => infer R ? (...args: A) => Promise<Awaited<R>> : T[K];
} & {
  use<R>(fn: (app: T) => R): Promise<Awaited<R>>;
};

export const wasm: Async<EditorEngine> = new Proxy({} as Async<EditorEngine>, {
  get: (_, prop: string | symbol) => {
    if (prop === 'use') return use;
    return async (...args: unknown[]) =>
      use((app) => (app as unknown as Record<string | symbol, (...a: unknown[]) => unknown>)[prop](...args));
  },
});
