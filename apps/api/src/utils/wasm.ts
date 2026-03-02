import { readFile } from 'node:fs/promises';
import type { Application } from '@typie/editor';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const base = import.meta.resolve!('@typie/editor');
const WASM_PATH = new URL('editor_bg.wasm', base).pathname;
const GLUE_PATH = new URL('editor_bg.js', base).pathname;

const POOL_SIZE = 10;

type WasmExports = {
  memory: WebAssembly.Memory;
  __wbindgen_start: () => void;
  [key: string]: unknown;
};

type GlueModule = {
  Application: new () => Application;
  __wbg_set_wasm: (exports: WasmExports) => void;
};

const glueSource = await readFile(GLUE_PATH, 'utf8');
const strippedGlueSource = glueSource.replaceAll(/^export /gm, '');

const glueExportNames: string[] = [];
const exportRe = /^export (?:function|class)\s+(\w+)/gm;
let match;
while ((match = exportRe.exec(glueSource)) !== null) {
  glueExportNames.push(match[1]);
}

function createGlueInstance(): GlueModule {
  const factory = new Function(`"use strict";\n${strippedGlueSource}\nreturn{${glueExportNames.join(',')}};`);
  return factory() as GlueModule;
}

async function createInstance(module: WebAssembly.Module): Promise<Application> {
  const glue = createGlueInstance();

  const instance = (await WebAssembly.instantiate(module, {
    './editor_bg.js': glue as unknown as WebAssembly.ModuleImports,
  })) as unknown as WebAssembly.Instance;

  const exports = instance.exports as WasmExports;
  glue.__wbg_set_wasm(exports);
  exports.__wbindgen_start();

  return new glue.Application();
}

const available: Application[] = [];
const waiting: ((app: Application) => void)[] = [];
let poolReady: Promise<void> | null = null;
let wasmModule: WebAssembly.Module | null = null;

async function initPool(): Promise<void> {
  const wasmBuffer = await readFile(WASM_PATH);
  wasmModule = await WebAssembly.compile(wasmBuffer);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const instances = await Promise.all(Array.from({ length: POOL_SIZE }, () => createInstance(wasmModule!)));
  available.push(...instances);
}

function returnToPool(app: Application): void {
  const next = waiting.shift();
  if (next) next(app);
  else available.push(app);
}

async function use<T>(fn: (app: Application) => T): Promise<Awaited<T>> {
  if (!poolReady) {
    poolReady = initPool().catch((err) => {
      poolReady = null;
      throw err;
    });
  }
  await poolReady;

  const app = available.pop() ?? (await new Promise<Application>((resolve) => waiting.push(resolve)));
  try {
    const result = await fn(app);
    returnToPool(app);
    return result;
  } catch (err) {
    if (err instanceof WebAssembly.RuntimeError && wasmModule) {
      createInstance(wasmModule).then(returnToPool, () => {
        /* noop*/
      });
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

export const wasm: Async<Application> = new Proxy({} as Async<Application>, {
  get: (_, prop: string | symbol) => {
    if (prop === 'use') return use;
    return async (...args: unknown[]) =>
      use((app) => (app as unknown as Record<string | symbol, (...a: unknown[]) => unknown>)[prop](...args));
  },
});
