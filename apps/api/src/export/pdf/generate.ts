import { parseVectorPageBinary } from '../core/codec.ts';
import { computeDesiredSize, resolveAssets } from '../core/external.ts';
import { nearestWeight } from '../core/fonts.ts';
import { SlateReader } from '../core/slate.ts';
import { LIGHT_THEME } from '../core/theme.ts';
import { ensureInstanceReady, SCALE_FACTOR, wasm } from '../core/wasm.ts';
import { ensureRequiredFont, filterUncoveredCodepoints, resolveFallbackMappings } from './fonts.ts';
import { createPdfFromVectorPages } from './index.ts';
import type { Asset } from '../core/external.ts';
import type { FontFamily } from './fonts.ts';

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
        theme: LIGHT_THEME,
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
        const font = nearestWeight(familyFonts, req.weight);
        tasks.push(
          (async () => {
            if (font) {
              await ensureRequiredFont(wasm, req.family, font, req.codepoints);
            }

            const uncovered = font ? await filterUncoveredCodepoints(font, req.codepoints) : req.codepoints;
            const coveredSet = new Set(uncovered);
            const covered = req.codepoints.filter((cp) => !coveredSet.has(cp));

            const mappings: { family: string; weight: number; codepoints: number[] }[] = [];
            if (font && covered.length > 0) {
              mappings.push({ family: req.family, weight: font.weight, codepoints: covered });
            }

            if (uncovered.length > 0) {
              const fallbackMappings = await resolveFallbackMappings(wasm, req.weight, uncovered);
              mappings.push(...fallbackMappings);
            }

            editor.dispatch({ type: 'fontsLoaded', family: req.family, weight: req.weight, mappings });
          })(),
        );
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
