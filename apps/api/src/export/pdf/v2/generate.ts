import { useHost } from '../../../utils/wasm-ffi-host.ts';
import { parseVectorPageBinary } from '../../core/codec.ts';
import { computeDesiredSize, resolveAssets } from '../../core/external.ts';
import { createPdfFromVectorPages } from '../index.ts';
import { mapExternalElement } from './external.ts';
import { buildEditorFontFamilies } from './font-families.ts';
import { handleFontDataMissing, registerFonts } from './fonts.ts';
import type { Editor, EditorEvent } from '@typie/editor-ffi/server';
import type { VectorPage } from '../../core/codec.ts';

const MAX_FONT_PASSES = 10;

export type GenerateDocumentPdfV2Params = {
  graph: Uint8Array;
  userId: string;
  title: string;
  author: string;
  layout: {
    pageWidth: number;
    pageHeight: number;
    pageMarginTop: number;
    pageMarginBottom: number;
    pageMarginLeft: number;
    pageMarginRight: number;
  };
};

export async function generateDocumentPdfV2(params: GenerateDocumentPdfV2Params): Promise<Uint8Array> {
  const { graph, userId, title, author, layout } = params;
  const families = await buildEditorFontFamilies(userId);

  return useHost(async (host) => {
    const reg = registerFonts(host, families);
    const viewport = { width: layout.pageWidth, height: layout.pageHeight, scale_factor: 1 };

    let editor: Editor;
    try {
      editor = host.create_editor_from_graph(graph, viewport);
    } catch (err) {
      if (String(err).includes('NoInitialCursorPosition')) {
        const blank: VectorPage = { width: layout.pageWidth, height: layout.pageHeight, ops: [], textOps: [] };
        return await createPdfFromVectorPages([blank], title, author, [], new Map());
      }
      throw err;
    }

    try {
      editor.enqueue({ type: 'system', event: { type: 'initialize' } });
      editor.enqueue({
        type: 'node',
        op: {
          type: 'set_attrs',
          id: '0',
          attrs: {
            type: 'root',
            layout_mode: {
              type: 'paginated',
              page_width: layout.pageWidth,
              page_height: layout.pageHeight,
              page_margin_top: layout.pageMarginTop,
              page_margin_bottom: layout.pageMarginBottom,
              page_margin_left: layout.pageMarginLeft,
              page_margin_right: layout.pageMarginRight,
            },
          },
        },
      });

      let loadedThisRound = false;
      for (let round = 0; round < MAX_FONT_PASSES; round++) {
        const events = editor.tick();
        const missing = events.filter((e): e is Extract<EditorEvent, { type: 'font_data_missing' }> => e.type === 'font_data_missing');
        if (missing.length === 0) {
          loadedThisRound = false;
          break;
        }
        loadedThisRound = true;
        await Promise.all(missing.map((e) => handleFontDataMissing(host, editor, reg, e)));
      }
      if (loadedThisRound) {
        const finalEvents = editor.tick();
        if (finalEvents.some((e) => e.type === 'font_data_missing')) {
          console.warn('[pdf-v2] font resolution did not converge; degraded export');
        }
      }

      const externals = editor.external_elements().map(mapExternalElement);
      if (externals.length > 0) {
        const assets = await resolveAssets(externals);
        for (const ext of externals) {
          const { height } = computeDesiredSize(ext, assets.get(ext.nodeId));
          editor.enqueue({ type: 'system', event: { type: 'set_external_height', node_id: ext.nodeId, height } });
        }
        editor.tick();
      }

      const sizes = editor.page_sizes();
      if (sizes.length === 0) {
        const blank: VectorPage = { width: layout.pageWidth, height: layout.pageHeight, ops: [], textOps: [] };
        return await createPdfFromVectorPages([blank], title, author, [], new Map());
      }
      const pages = sizes.map((_, i) => parseVectorPageBinary(editor.export_page_vector(i, 1)));

      const finalExternals = editor.external_elements().map(mapExternalElement);
      const finalAssets = await resolveAssets(finalExternals);
      return await createPdfFromVectorPages(pages, title, author, finalExternals, finalAssets);
    } finally {
      editor.free();
    }
  });
}
