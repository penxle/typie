import { readFile } from 'node:fs/promises';
import path from 'node:path';
import type { Application } from '@typie/editor';

const editorPkgDir = path.dirname(Bun.resolveSync('@typie/editor/pkg/editor.js', import.meta.dir));

const WASM_PATH = path.join(editorPkgDir, 'editor_bg.wasm');
const GLUE_PATH = path.join(editorPkgDir, 'editor_bg.js');
const ICU_DATA_PATH = path.join(editorPkgDir, 'icu_data.postcard');

type WasmExports = {
  memory: WebAssembly.Memory;
  __wbindgen_start: () => void;
  [key: string]: unknown;
};

type GlueModule = {
  Application: new () => Application;
  snapshotToJson: (snapshot: Uint8Array) => unknown;
  jsonToSnapshot: (json: unknown) => Uint8Array;
  getMemory: () => WebAssembly.Memory;
  __wbg_set_wasm: (exports: WasmExports) => void;
};

let glueModule: GlueModule | null = null;

async function getGlue(): Promise<GlueModule> {
  if (glueModule) {
    return glueModule;
  }

  const wasmBuffer = await readFile(WASM_PATH);
  const module = await WebAssembly.compile(wasmBuffer);
  const glue = (await import(GLUE_PATH)) as unknown as GlueModule;

  const instance = (await WebAssembly.instantiate(module, {
    './editor_bg.js': glue as unknown as WebAssembly.ModuleImports,
  })) as unknown as WebAssembly.Instance;

  const exports = instance.exports as WasmExports;
  glue.__wbg_set_wasm(exports);
  exports.__wbindgen_start();

  glueModule = glue;
  return glue;
}

let icuData: Uint8Array | null = null;

async function getIcuData(): Promise<Uint8Array> {
  if (!icuData) {
    icuData = new Uint8Array(await readFile(ICU_DATA_PATH));
  }
  return icuData;
}

export async function snapshotToJson(snapshot: Uint8Array): Promise<unknown> {
  const glue = await getGlue();
  return glue.snapshotToJson(snapshot);
}

export async function jsonToSnapshot(json: unknown): Promise<Uint8Array> {
  const glue = await getGlue();
  return glue.jsonToSnapshot(json);
}

export async function createWasmApplication(): Promise<{
  app: Application;
  getMemory: () => WebAssembly.Memory;
  icuData: Uint8Array;
  cleanup: () => void;
}> {
  const [glue, icu] = await Promise.all([getGlue(), getIcuData()]);
  const app = new glue.Application();

  return {
    app,
    getMemory: glue.getMemory,
    icuData: icu,
    cleanup: () => {
      app.free();
    },
  };
}
