import { parseVectorPageBinary } from '../core/codec.ts';
import { computeDesiredSize, resolveAssets } from '../core/external.ts';
import { nearestWeight } from '../core/fonts.ts';
import { SlateReader } from '../core/slate.ts';
import { DARK_THEME, LIGHT_THEME } from '../core/theme.ts';
import { ensureInstanceReady, SCALE_FACTOR, wasm } from '../core/wasm.ts';
import { ensureRequiredFont, filterUncoveredCodepoints, resolveFallbackMappings } from '../pdf/fonts.ts';
import { renderPreviewImage } from './layout.tsx.js';
import { buildPageSvg } from './svg.ts';
import type { Asset } from '../core/external.ts';
import type { FontFamily } from '../pdf/fonts.ts';

const DEFAULT_PAGE_WIDTH = 665;
const DEFAULT_PAGE_HEIGHT = 945;
const DEFAULT_PAGE_MARGIN = 72;

export type PreviewTheme = 'light' | 'dark';

export type GeneratePreviewParams = {
  snapshot: Uint8Array;
  title: string;
  subtitle: string | null;
  fonts: FontFamily[];
  width: number;
  theme: PreviewTheme;
};

export async function generateDocumentPreview(params: GeneratePreviewParams): Promise<Uint8Array> {
  const { snapshot, title, subtitle, fonts, width, theme } = params;

  const layout = {
    pageWidth: DEFAULT_PAGE_WIDTH,
    pageHeight: DEFAULT_PAGE_HEIGHT,
    pageMarginTop: DEFAULT_PAGE_MARGIN,
    pageMarginBottom: DEFAULT_PAGE_MARGIN,
    pageMarginLeft: DEFAULT_PAGE_MARGIN,
    pageMarginRight: DEFAULT_PAGE_MARGIN,
  };

  return wasm.use(async (app) => {
    await ensureInstanceReady(app);

    const editor = app.createEditor(SCALE_FACTOR, snapshot);

    try {
      editor.dispatch({
        type: 'initialize',
        theme: theme === 'dark' ? DARK_THEME : LIGHT_THEME,
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

      editor.setMaxPages(1);
      editor.setAllFoldsExpanded(true);

      const offsets = Object.fromEntries(editor.getSlateOffsets());
      const memory = app.getMemory() as WebAssembly.Memory;
      const slate = new SlateReader(memory, offsets, editor.getSlatePtr(), editor.getSlabPtr());

      // Tick 1
      editor.tick();
      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());

      const tasks: Promise<void>[] = [];

      // 폰트 요청
      for (const req of slate.readFontRequests()) {
        const familyFonts = fonts.find((f) => f.familyName === req.family)?.fonts ?? [];
        const font = nearestWeight(familyFonts, req.weight);
        tasks.push(
          (async () => {
            if (font) {
              await ensureRequiredFont(app, req.family, font, req.codepoints);
            }

            const uncovered = font ? await filterUncoveredCodepoints(font, req.codepoints) : req.codepoints;
            const coveredSet = new Set(uncovered);
            const covered = req.codepoints.filter((cp) => !coveredSet.has(cp));

            const mappings: { family: string; weight: number; codepoints: number[] }[] = [];
            if (font && covered.length > 0) {
              mappings.push({ family: req.family, weight: font.weight, codepoints: covered });
            }

            if (uncovered.length > 0) {
              const fallbackMappings = await resolveFallbackMappings(app, req.weight, uncovered);
              mappings.push(...fallbackMappings);
            }

            editor.dispatch({ type: 'fontsLoaded', family: req.family, weight: req.weight, mappings });
          })(),
        );
      }

      // 외부 에셋
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

      // Tick 2
      editor.tick();
      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());

      if (slate.pagesCount === 0) {
        throw new Error('Empty document');
      }

      // 첫 페이지만 추출
      const bytes = editor.exportPageVector(0);
      if (!bytes) {
        throw new Error('Missing vector page payload');
      }
      const page = parseVectorPageBinary(bytes);

      slate.refresh(editor.getSlatePtr(), editor.getSlabPtr());
      const laidExternals = slate.readExternalElements();

      // SVG 조립
      const bodySvg = await buildPageSvg(page, laidExternals, assets, {
        top: layout.pageMarginTop,
        bottom: layout.pageMarginBottom,
        left: layout.pageMarginLeft,
        right: layout.pageMarginRight,
      });

      // satori 레이아웃 + 래스터화
      return await renderPreviewImage({ title, subtitle, bodySvg, theme }, width);
    } finally {
      editor.free();
    }
  });
}
