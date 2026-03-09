import { readFile } from 'node:fs/promises';
import { wasm } from '@/utils/wasm';
import { computeDesiredSize, resolveAssets } from './external';
import { createPdfFromVectorPages } from './pdf';
import { parseVectorPageBinary } from './pdf/codec';
import { ensureRequiredFallbackFont, ensureRequiredFont, filterUncoveredCodepoints, initFonts } from './pdf/fonts';
import { SlateReader } from './pdf/slate';
import { DEFAULT_THEME } from './theme';
import type { Application } from '@typie/editor';
import type { Asset } from './external';
import type { FontFamily } from './pdf/fonts';

export type { GenerateDocumentDocxParams } from './docx';
export { generateDocumentDocx } from './docx';
export type { GenerateDocumentEpubParams } from './epub';
export { generateDocumentEpub } from './epub';
export type { GenerateDocumentHwpParams } from './hwp';
export { generateDocumentHwp } from './hwp';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const ICU_DATA_PATH = new URL(import.meta.resolve!('@typie/editor/icu/data.postcard')).pathname;
let icuDataPromise: Promise<Uint8Array> | null = null;
function getIcuData(): Promise<Uint8Array> {
  return (icuDataPromise ??= readFile(ICU_DATA_PATH).then((buf) => new Uint8Array(buf)));
}

const initialized = new WeakMap<Application, Promise<void>>();
async function ensureInstanceReady(app: Application): Promise<void> {
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

const SCALE_FACTOR = 2;

export type GenerateDocumentPdfParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  fonts: FontFamily[];
  layout: {
    pageWidth: number;
    pageHeight: number;
    pageMarginTop: number;
    pageMarginBottom: number;
    pageMarginLeft: number;
    pageMarginRight: number;
  };
};

export async function generateDocumentPdf(params: GenerateDocumentPdfParams): Promise<Uint8Array> {
  const { snapshot, title, author, fonts, layout } = params;
  return wasm.use(async (wasm) => {
    await ensureInstanceReady(wasm);

    const editor = wasm.createEditor(SCALE_FACTOR, snapshot);

    try {
      editor.dispatch({
        type: 'initialize',
        theme: DEFAULT_THEME,
        viewportWidth: layout.pageWidth,
        viewportHeight: layout.pageHeight,
        scaleFactor: SCALE_FACTOR,
      });

      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          type: 'paginated',
          pageWidth: layout.pageWidth,
          pageHeight: layout.pageHeight,
          pageMarginTop: layout.pageMarginTop,
          pageMarginBottom: layout.pageMarginBottom,
          pageMarginLeft: layout.pageMarginLeft,
          pageMarginRight: layout.pageMarginRight,
        },
      });

      editor.setAllFoldsExpanded(true);

      const offsets = Object.fromEntries(editor.getSlateOffsets());
      const memory = wasm.getMemory() as WebAssembly.Memory;
      const slate = new SlateReader(memory, offsets, editor.getSlatePtr(), editor.getSlabPtr());

      // Tick 1: 스냅샷/상태 변경 처리, 필요한 폰트·외부 요소 목록 산출
      editor.tick();
      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());

      const tasks: Promise<void>[] = [];

      // 폰트 요청 파싱 & 로드
      for (const req of slate.readFontRequests()) {
        const familyFonts = fonts.find((f) => f.familyName === req.family)?.fonts ?? [];
        // see: Rust nearest_weight()
        const font =
          familyFonts.find((f) => f.weight === req.weight) ??
          familyFonts.reduce<(typeof familyFonts)[number] | null>((prev, curr) => {
            if (!prev) return curr;
            const prevDiff = Math.abs(prev.weight - req.weight);
            const currDiff = Math.abs(curr.weight - req.weight);
            if (currDiff < prevDiff) return curr;
            if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
            return prev;
          }, null);
        if (font) {
          tasks.push(
            Promise.all([
              ensureRequiredFont(wasm, req.family, font, req.codepoints),
              filterUncoveredCodepoints(font, req.codepoints).then((uncovered) =>
                uncovered.length > 0 ? ensureRequiredFallbackFont(wasm, req.weight, uncovered) : undefined,
              ),
            ]).then(() => {
              editor.dispatch({ type: 'fontsLoaded', family: req.family, weight: req.weight, codepoints: req.codepoints });
            }),
          );
        } else {
          tasks.push(
            ensureRequiredFallbackFont(wasm, req.weight, req.codepoints).then(() => {
              editor.dispatch({ type: 'fontsLoaded', family: req.family, weight: req.weight, codepoints: req.codepoints });
            }),
          );
        }
      }

      // 외부 요소 asset 해석 & 높이 보정
      let assets = new Map<string, Asset>();
      const externals = slate.readExternalElements();
      tasks.push(
        resolveAssets(externals).then((resolved) => {
          assets = resolved;
          for (const ext of externals) {
            const { height } = computeDesiredSize(ext, assets.get(ext.nodeId));
            editor.dispatch({ type: 'setExternalElementHeight', nodeId: ext.nodeId, height });
          }
        }),
      );

      await Promise.all(tasks);

      // Tick 2: 폰트 + 외부 요소 높이 반영 후 최종 레이아웃
      editor.tick();
      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());

      const pageCount = slate.pagesCount;
      if (pageCount === 0) {
        throw new Error('Failed to layout document');
      }

      const pages = Array.from({ length: pageCount }, (_, i) => {
        const bytes = editor.exportPageVector(i);
        if (!bytes) {
          throw new Error(`Missing vector page payload for page index ${i}`);
        }

        return parseVectorPageBinary(bytes);
      });

      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());
      const laidExternals = slate.readExternalElements();
      return await createPdfFromVectorPages(pages, title, author, laidExternals, assets);
    } finally {
      editor.free();
    }
  });
}
