import { extractAssetIdsFromPlainDoc } from '#/utils/entity.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { loadEmbeds, loadImageAssets } from '../assets.ts';
import type { PlainDoc, PlainRootNode } from '@typie/editor-ffi/server';
import type { PageLayout } from '../types.ts';
import type { DocDefaultsV2, ParsedDocumentV2 } from './types.ts';

const PX_PER_UNIT = 16 / 100;
function layoutFromRoot(root: PlainRootNode): PageLayout | undefined {
  const m = root.layout_mode;
  if (m.type !== 'paginated') return undefined;
  return {
    pageWidth: m.page_width,
    pageHeight: m.page_height,
    pageMarginTop: m.page_margin_top,
    pageMarginBottom: m.page_margin_bottom,
    pageMarginLeft: m.page_margin_left,
    pageMarginRight: m.page_margin_right,
  };
}
export async function parseDocumentV2(graph: Uint8Array): Promise<ParsedDocumentV2> {
  const plain = (await wasm.to_plain_resolved(graph)) as PlainDoc;
  const rootEntry = plain.root;
  if (rootEntry.node.type !== 'root') throw new Error('Root node not found in resolved PlainDoc');
  const rootMods = rootEntry.modifiers;
  const ff = rootMods['font_family'];
  const fs = rootMods['font_size'];
  const lh = rootMods['line_height'];
  const pi = rootMods['paragraph_indent'];
  const bg = rootMods['block_gap'];
  const defaults: DocDefaultsV2 = {
    fontFamily: ff?.type === 'font_family' ? ff.value : 'Pretendard',
    fontSizePt100: fs?.type === 'font_size' ? fs.value : 1200,
    lineHeight: lh?.type === 'line_height' ? lh.value : 160,
    paragraphIndentPx: (pi?.type === 'paragraph_indent' ? pi.value : 100) * PX_PER_UNIT,
    blockGapPx: (bg?.type === 'block_gap' ? bg.value : 100) * PX_PER_UNIT,
  };
  const { imageIds, embedIds } = extractAssetIdsFromPlainDoc(plain);
  const [images, embeds] = await Promise.all([loadImageAssets(imageIds), loadEmbeds(embedIds)]);
  return { plain, root: rootEntry, defaults, layout: layoutFromRoot(rootEntry.node as PlainRootNode), images, embeds };
}
