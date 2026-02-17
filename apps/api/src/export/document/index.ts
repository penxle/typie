import { readFile } from 'node:fs/promises';
import { wasm } from '@/utils/wasm';
import { ensureRequiredFallbackFont, ensureRequiredFont, filterUncoveredCodepoints, initFonts } from './fonts';
import { createPdfFromPages } from './pdf';
import { renderDocumentPages } from './render';
import type { Application, Theme } from '@typie/editor';
import type { FontFamily } from './fonts';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const ICU_DATA_PATH = new URL('icu_data.postcard', import.meta.resolve!('@typie/editor')).pathname;
let icuDataCache: Uint8Array | null = null;
async function getIcuData(): Promise<Uint8Array> {
  if (!icuDataCache) icuDataCache = new Uint8Array(await readFile(ICU_DATA_PATH));
  return icuDataCache;
}

const initialized = new WeakSet<Application>();
async function ensureInstanceReady(app: Application): Promise<void> {
  if (initialized.has(app)) return;
  app.loadIcuData(await getIcuData());
  await initFonts(app);
  initialized.add(app);
}

const SCALE_FACTOR = 2;
const MAX_TICKS = 1000;
const MM_TO_PX = 96 / 25.4;

const DEFAULT_THEME: Theme = {
  colors: new Map([
    ['text.default', 0x21_25_29_ff],
    ['text.subtle', 0x49_50_57_ff],
    ['text.muted', 0x86_8e_96_ff],
    ['background.default', 0xf8_f9_fa_ff],
    ['border.default', 0xde_e2_e6_ff],
    ['accent.blue.default', 0x22_8b_e6_ff],
    ['accent.red.default', 0xfa_52_52_ff],
    ['accent.green.default', 0x40_c0_57_ff],
    ['accent.yellow.default', 0xfa_b0_05_ff],
    ['accent.purple.default', 0xbe_4b_db_ff],
    ['accent.teal.default', 0x12_b8_86_ff],
    ['accent.gray.default', 0x86_8e_96_ff],
  ]),
};

export type PageLayout = {
  width: number;
  height: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
};

export type GenerateDocumentPdfParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  fonts: FontFamily[];
  pageLayout: PageLayout;
  timeout?: number;
};

export async function generateDocumentPdf(params: GenerateDocumentPdfParams): Promise<Uint8Array> {
  const { snapshot, title, author, fonts, pageLayout, timeout = 30_000 } = params;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const result = await Promise.race([
      generateDocumentPdfInternal(snapshot, title, author, fonts, pageLayout),
      new Promise<never>((_, reject) => {
        controller.signal.addEventListener('abort', () => {
          reject(new Error('PDF generation timed out'));
        });
      }),
    ]);
    return result;
  } finally {
    clearTimeout(timeoutId);
  }
}

async function generateDocumentPdfInternal(
  snapshot: Uint8Array,
  title: string,
  author: string,
  fonts: FontFamily[],
  pageLayout: PageLayout,
): Promise<Uint8Array> {
  return wasm.use(async (wasm) => {
    await ensureInstanceReady(wasm);

    const editor = wasm.createEditor(SCALE_FACTOR, snapshot);

    try {
      editor.dispatch({
        type: 'initialize',
        theme: DEFAULT_THEME,
      });

      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          type: 'paginated',
          pageWidth: pageLayout.width * MM_TO_PX,
          pageHeight: pageLayout.height * MM_TO_PX,
          pageMarginTop: pageLayout.marginTop * MM_TO_PX,
          pageMarginBottom: pageLayout.marginBottom * MM_TO_PX,
          pageMarginLeft: pageLayout.marginLeft * MM_TO_PX,
          pageMarginRight: pageLayout.marginRight * MM_TO_PX,
        },
      });

      let pageCount = 0;
      let needsRender = false;

      const offsets = Object.fromEntries(editor.getSlateOffsets());
      const getMemory = () => wasm.getMemory() as WebAssembly.Memory;
      const memory = getMemory();

      const DIRTY_PAGES = 1;
      const DIRTY_RENDER_REQUIRED = 16;
      const DIRTY_FONT_REQUIRED = 17;

      for (let tick = 0; tick < MAX_TICKS; tick++) {
        editor.tick();
        editor.flush();

        const view = new DataView(memory.buffer);
        const slatePtr = editor.getSlatePtr();
        const slabPtr = editor.getSlabPtr();

        const dirtyLo = view.getUint32(slatePtr + offsets.dirty, true);

        if (dirtyLo === 0) {
          if (needsRender && pageCount > 0) {
            break;
          }
          continue;
        }

        const fontPromises: Promise<void>[] = [];

        if (dirtyLo & (1 << DIRTY_PAGES)) {
          pageCount = view.getUint32(slatePtr + offsets.pages_count, true);
        }

        if (dirtyLo & (1 << DIRTY_FONT_REQUIRED)) {
          const count = view.getUint32(slatePtr + offsets.font_requests_count, true);
          let pos = slabPtr + view.getUint32(slatePtr + offsets.font_requests_offset, true);
          for (let i = 0; i < count; i++) {
            const byteLen = view.getUint32(pos, true);
            const family = new TextDecoder().decode(new Uint8Array(memory.buffer, pos + 4, byteLen));
            pos += 4 + ((byteLen + 3) & ~3);
            const weight = view.getUint32(pos, true);
            pos += 4;
            const cpCount = view.getUint32(pos, true);
            pos += 4;
            const codepoints: number[] = [];
            for (let j = 0; j < cpCount; j++) {
              codepoints.push(view.getUint32(pos + j * 4, true));
            }
            pos += cpCount * 4;
            const font = fonts.find((f) => f.familyName === family)?.fonts.find((f) => f.weight === weight);
            if (font) {
              fontPromises.push(
                ensureRequiredFont(wasm, family, font, codepoints),
                filterUncoveredCodepoints(font, codepoints).then((uncovered) =>
                  uncovered.length > 0 ? ensureRequiredFallbackFont(wasm, weight, uncovered) : undefined,
                ),
              );
            } else {
              fontPromises.push(ensureRequiredFallbackFont(wasm, weight, codepoints));
            }
          }
        }

        if (dirtyLo & (1 << DIRTY_RENDER_REQUIRED)) {
          needsRender = true;
        }

        if (fontPromises.length > 0) {
          await Promise.all(fontPromises);
          editor.dispatch({ type: 'fontsLoaded' });
        }

        if (needsRender && pageCount > 0) {
          break;
        }
      }

      if (pageCount === 0) {
        throw new Error('Failed to layout document');
      }

      const pages = await renderDocumentPages(editor, getMemory, pageCount);

      return await createPdfFromPages(pages, SCALE_FACTOR, title, author);
    } finally {
      editor.free();
    }
  });
}
