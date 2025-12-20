import { readFile } from 'node:fs/promises';
import path from 'node:path';
import type { Application } from '@typie/editor';

const editorPkgDir = path.dirname(Bun.resolveSync('@typie/editor/pkg/editor.js', import.meta.dir));

const WASM_PATH = path.join(editorPkgDir, 'editor_bg.wasm');
const GLUE_PATH = path.join(editorPkgDir, 'editor_bg.js');
const ICU_DATA_PATH = path.join(editorPkgDir, 'icu_data.postcard');
const PHANTOM_FONT_PATH = path.join(editorPkgDir, 'Noto-Phantom.ttf');

let wasmModule: WebAssembly.Module | null = null;
let icuData: Uint8Array | null = null;
let phantomFont: Uint8Array | null = null;

async function loadResources() {
  if (!wasmModule) {
    const wasmBuffer = await readFile(WASM_PATH);
    wasmModule = await WebAssembly.compile(wasmBuffer);
  }
  if (!icuData) {
    icuData = new Uint8Array(await readFile(ICU_DATA_PATH));
  }
  if (!phantomFont) {
    phantomFont = new Uint8Array(await readFile(PHANTOM_FONT_PATH));
  }

  return { wasmModule, icuData, phantomFont };
}

type WasmExports = {
  memory: WebAssembly.Memory;
  __wbindgen_start: () => void;
  [key: string]: unknown;
};

type GlueModule = {
  Application: new () => Application;
  getMemory: () => WebAssembly.Memory;
  __wbg_set_wasm: (exports: WasmExports) => void;
};

export async function createWasmApplication(): Promise<{
  app: Application;
  getMemory: () => WebAssembly.Memory;
  icuData: Uint8Array;
  phantomFont: Uint8Array;
  cleanup: () => void;
}> {
  const resources = await loadResources();

  const glue = (await import(GLUE_PATH)) as unknown as GlueModule;

  const instance = await WebAssembly.instantiate(resources.wasmModule, {
    './editor_bg.js': glue as unknown as WebAssembly.ModuleImports,
  });

  const exports = instance.exports as WasmExports;
  glue.__wbg_set_wasm(exports);
  exports.__wbindgen_start();

  const app = new glue.Application();

  return {
    app,
    getMemory: glue.getMemory,
    icuData: resources.icuData,
    phantomFont: resources.phantomFont,
    cleanup: () => {
      app.free();
    },
  };
}
