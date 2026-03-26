import { readFile } from 'node:fs/promises';
import { initFonts } from '../pdf/fonts.ts';
import type { EditorEngine } from '@typie/editor';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const ICU_DATA_PATH = new URL(import.meta.resolve!('@typie/editor/icu.zst')).pathname;
let icuDataPromise: Promise<Uint8Array> | null = null;
function getIcuData(): Promise<Uint8Array> {
  return (icuDataPromise ??= readFile(ICU_DATA_PATH).then((buf) => new Uint8Array(buf)));
}

const initialized = new WeakMap<EditorEngine, Promise<void>>();
export async function ensureInstanceReady(app: EditorEngine): Promise<void> {
  let pending = initialized.get(app);
  if (!pending) {
    pending = (async () => {
      app.loadIcuData(await getIcuData());
      await initFonts(app);
    })().catch((err) => {
      initialized.delete(app);
      throw err;
    });
    initialized.set(app, pending);
  }
  await pending;
}

export const SCALE_FACTOR = 2;

export { wasm } from '#/utils/wasm.ts';
