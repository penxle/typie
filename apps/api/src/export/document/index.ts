import { ensureRequiredFonts, getAvailableFontsMap, loadInitialFonts } from './fonts';
import { createPdfFromPages } from './pdf';
import { renderDocumentPages } from './render';
import { createWasmApplication } from './wasm';
import type { Cmd, Theme } from '@typie/editor';

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
  pageLayout: PageLayout;
  timeout?: number;
};

export async function generateDocumentPdf(params: GenerateDocumentPdfParams): Promise<Uint8Array> {
  const { snapshot, title, author, pageLayout, timeout = 30_000 } = params;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const result = await Promise.race([
      generateDocumentPdfInternal(snapshot, title, author, pageLayout),
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
  pageLayout: PageLayout,
): Promise<Uint8Array> {
  const { app, getMemory, icuData, phantomFont, cleanup } = await createWasmApplication();

  try {
    app.loadIcuData(icuData);
    app.registerFallbackFont('Noto-Phantom', 400, phantomFont);
    app.setAvailableFonts(getAvailableFontsMap());

    await loadInitialFonts(app);

    const editor = app.createEditor(SCALE_FACTOR, snapshot);

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
      let pendingFonts: [string, number][] = [];
      let needsRender = false;

      for (let tick = 0; tick < MAX_TICKS; tick++) {
        const cmds = editor.tick() as Cmd[] | null;
        editor.flush();

        if (!cmds || cmds.length === 0) {
          if (needsRender && pageCount > 0) {
            break;
          }
          continue;
        }

        for (const cmd of cmds) {
          switch (cmd.type) {
            case 'layoutChanged': {
              pageCount = cmd.pageCount;
              break;
            }
            case 'fontsRequired': {
              pendingFonts.push(...cmd.fonts);
              break;
            }
            case 'renderRequired': {
              needsRender = true;
              break;
            }
          }
        }

        if (pendingFonts.length > 0) {
          await ensureRequiredFonts(app, pendingFonts);
          pendingFonts = [];
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
  } finally {
    cleanup();
  }
}
