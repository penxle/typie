import { readFile } from 'node:fs/promises';
import { wasm } from '@/utils/wasm';
import { resolveExternalImages } from './external';
import { ensureRequiredFallbackFont, ensureRequiredFont, filterUncoveredCodepoints, initFonts } from './fonts';
import { createPdfFromVectorPages } from './pdf';
import { exportDocumentVectorPages } from './vector';
import type { Application, Theme } from '@typie/editor';
import type { ResolvedExternalImage } from './external';
import type { FontFamily } from './fonts';
import type { VectorExternalElement, VectorPage } from './vector';

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
const EXTERNAL_FALLBACK_HEIGHT = 48;
const HEIGHT_EPSILON = 0.5;
const MAX_EXTERNAL_LAYOUT_PASSES = 3;

const colorToU32 = (color: string): number => {
  const clean = color.replace('#', '');
  if (clean.length !== 6) {
    return 0x00_00_00_ff;
  }
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return ((r << 24) | (g << 16) | (b << 8) | 0xff) >>> 0;
};

const DEFAULT_THEME_COLORS: Record<string, string> = {
  'ui.surface.default': '#ffffff',
  'ui.surface.subtle': '#fafafa',
  'ui.surface.muted': '#f4f4f5',
  'ui.surface.dark': '#3f3f46',
  'ui.text.default': '#18181b',
  'ui.text.subtle': '#3f3f46',
  'ui.text.muted': '#52525c',
  'ui.text.faint': '#71717b',
  'ui.text.disabled': '#9f9fa9',
  'ui.text.bright': '#ffffff',
  'ui.text.danger': '#fb2c36',
  'ui.text.success': '#008236',
  'ui.text.link': '#006cff',
  'ui.text.brand': '#fd9a00',
  'ui.interactive.hover': '#e4e4e7',
  'ui.interactive.disabled': '#e4e4e7',
  'ui.accent.brand.default': '#fd9a00',
  'ui.accent.brand.hover': '#e17100',
  'ui.accent.brand.active': '#bb4d00',
  'ui.accent.brand.subtle': '#fef3c6',
  'ui.accent.danger.default': '#e7000b',
  'ui.accent.danger.hover': '#fb2c36',
  'ui.accent.danger.active': '#c10007',
  'ui.accent.danger.subtle': '#fef2f2',
  'ui.accent.success.subtle': '#f0fdf4',
  'ui.border.default': '#e4e4e7',
  'ui.border.strong': '#d4d4d8',
  'ui.border.subtle': '#f4f4f5',
  'ui.border.brand': '#e17100',
  'ui.border.danger': '#e7000b',
  'ui.shadow.default': '#09090b',
  'ui.control.scrollbar.default': '#e4e4e7',
  'ui.control.scrollbar.hover': '#d4d4d8',
  'ui.decoration.grid.default': '#f4f4f5',
  'ui.decoration.grid.subtle': '#fafafa',
  'ui.decoration.grid.brand': '#fef3c6',
  'ui.decoration.grid.brand.subtle': '#fffbeb',
  'ui.callout.info': '#3b82f6',
  'ui.callout.success': '#22c55e',
  'ui.callout.warning': '#f97316',
  'ui.callout.danger': '#dc2626',
  'ui.blockquote.message-sent': '#248bf5',
  'ui.blockquote.message-received': '#e5e5ea',
  'text.bright': '#ffffff',
  'text.black': '#18181b',
  'text.darkgray': '#525254',
  'text.gray': '#8c8c8d',
  'text.lightgray': '#c5c5c6',
  'text.white': '#ffffff',
  'text.red': '#ef4444',
  'text.orange': '#f97316',
  'text.amber': '#f59e0b',
  'text.yellow': '#eab308',
  'text.lime': '#84cc16',
  'text.green': '#22c55e',
  'text.emerald': '#10b981',
  'text.teal': '#14b8a6',
  'text.cyan': '#06b6d4',
  'text.sky': '#0ea5e9',
  'text.blue': '#3b82f6',
  'text.indigo': '#6366f1',
  'text.violet': '#8b5cf6',
  'text.purple': '#a855f7',
  'text.fuchsia': '#d946ef',
  'text.pink': '#ec4899',
  'text.rose': '#f43f5e',
  'bg.gray': '#f1f1f2',
  'bg.red': '#fdebec',
  'bg.orange': '#ffecd5',
  'bg.yellow': '#fef3c7',
  'bg.green': '#dff3e3',
  'bg.blue': '#e7f3f8',
  'bg.purple': '#f0e7fe',
  selection: '#99ccff',
};

const DEFAULT_THEME: Theme = {
  colors: new Map(Object.entries(DEFAULT_THEME_COLORS).map(([key, value]) => [key, colorToU32(value)])),
};

const computeImageHeight = (external: VectorExternalElement, asset: ResolvedExternalImage | undefined): number => {
  if (external.data.type !== 'image') {
    return EXTERNAL_FALLBACK_HEIGHT;
  }

  if (!asset || asset.width <= 0 || asset.height <= 0) {
    return EXTERNAL_FALLBACK_HEIGHT;
  }

  const widthLimit = external.bounds.width * external.data.proportion;
  const renderedWidth = Math.min(asset.width, widthLimit);
  const aspectRatio = asset.height / asset.width;
  const renderedHeight = renderedWidth * aspectRatio;

  if (!Number.isFinite(renderedHeight) || renderedHeight <= 0) {
    return EXTERNAL_FALLBACK_HEIGHT;
  }

  return renderedHeight;
};

const applyExternalElementHeights = (
  editor: { dispatch: (message: { type: 'setExternalElementHeight'; nodeId: string; height: number }) => void },
  pages: VectorPage[],
  externalImages: ReadonlyMap<string, ResolvedExternalImage>,
): boolean => {
  const currentHeights = new Map<string, number>();
  const desiredHeights = new Map<string, number>();

  for (const page of pages) {
    for (const external of page.externalElements) {
      currentHeights.set(external.nodeId, external.bounds.height);

      if (external.data.type === 'image') {
        const asset = external.data.id ? externalImages.get(external.data.id) : undefined;
        desiredHeights.set(external.nodeId, computeImageHeight(external, asset));
        continue;
      }

      desiredHeights.set(external.nodeId, Math.max(external.bounds.height, EXTERNAL_FALLBACK_HEIGHT));
    }
  }

  let updated = false;
  for (const [nodeId, desiredHeight] of desiredHeights) {
    const currentHeight = currentHeights.get(nodeId) ?? 0;
    if (Math.abs(currentHeight - desiredHeight) <= HEIGHT_EPSILON) {
      continue;
    }

    editor.dispatch({
      type: 'setExternalElementHeight',
      nodeId,
      height: desiredHeight,
    });
    updated = true;
  }

  return updated;
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

      editor.setAllFoldsExpanded(true);

      const offsets = Object.fromEntries(editor.getSlateOffsets());
      const DIRTY_PAGES = 1;
      const DIRTY_RENDER_REQUIRED = 16;
      const DIRTY_FONT_REQUIRED = 17;
      const memory = wasm.getMemory() as WebAssembly.Memory;
      const textDecoder = new TextDecoder();

      const settleLayout = async (): Promise<number> => {
        let pageCount = 0;
        let needsRender = false;

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
          const loadedFontKeys = new Map<string, { family: string; weight: number }>();

          if (dirtyLo & (1 << DIRTY_PAGES)) {
            pageCount = view.getUint32(slatePtr + offsets.pages_count, true);
          }

          if (dirtyLo & (1 << DIRTY_FONT_REQUIRED)) {
            const count = view.getUint32(slatePtr + offsets.font_requests_count, true);
            let pos = slabPtr + view.getUint32(slatePtr + offsets.font_requests_offset, true);
            for (let i = 0; i < count; i++) {
              const byteLen = view.getUint32(pos, true);
              const family = textDecoder.decode(new Uint8Array(memory.buffer, pos + 4, byteLen));
              pos += 4 + ((byteLen + 3) & ~3);
              const weight = view.getUint32(pos, true);
              loadedFontKeys.set(`${family}:${weight}`, { family, weight });
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

          let dispatchedFontsLoaded = false;
          if (fontPromises.length > 0) {
            await Promise.all(fontPromises);
            for (const { family, weight } of loadedFontKeys.values()) {
              editor.dispatch({ type: 'fontsLoaded', family, weight });
              dispatchedFontsLoaded = true;
            }
          }

          if (dispatchedFontsLoaded) {
            continue;
          }

          if (needsRender && pageCount > 0) {
            break;
          }
        }

        if (pageCount === 0) {
          throw new Error('Failed to layout document');
        }

        return pageCount;
      };

      let pageCount = 0;
      let pages: VectorPage[] = [];
      let externalImages = new Map<string, ResolvedExternalImage>();

      for (let pass = 0; pass < MAX_EXTERNAL_LAYOUT_PASSES; pass++) {
        pageCount = await settleLayout();
        pages = exportDocumentVectorPages(editor, pageCount, offsets, memory);
        externalImages = await resolveExternalImages(pages);

        if (!applyExternalElementHeights(editor, pages, externalImages)) {
          break;
        }
      }

      return await createPdfFromVectorPages(pages, title, author, externalImages);
    } finally {
      editor.free();
    }
  });
}
