import { wasm } from '#/utils/wasm.ts';
import { collectNodeIds, loadEmbeds, loadImageAssets } from './assets.ts';
import type { DocumentJson, EmbedInfo, ImageAsset, NodeEntry } from './types.ts';

export type ParsedDocument = {
  nodes: Record<string, NodeEntry>;
  settings: Record<string, unknown>;
  defaults: {
    fontFamily: string;
    fontSizePt100: number;
    lineHeight: number;
    paragraphIndentPx: number;
    blockGapPx: number;
  };
  images: Map<string, ImageAsset>;
  embeds: Map<string, EmbedInfo>;
};

/**
 * snapshot → JSON 파싱 + 기본값 추출 + 에셋 로딩
 * 현재 hwp/docx/epub index.ts에 3회 복제된 로직을 통합
 */
export async function parseDocument(snapshot: Uint8Array): Promise<ParsedDocument> {
  const json = (await wasm.snapshotToJson(snapshot)) as unknown as DocumentJson;
  const nodes = json.nodes;

  // 루트 노드에서 기본 스타일 추출
  const rootId = Object.keys(nodes).find((id) => nodes[id].type === 'root');
  if (!rootId) throw new Error('Root node not found in snapshot');
  const rootEntry = nodes[rootId];
  const cascadeAttrs = rootEntry.cascade_attrs as Record<string, unknown> | undefined;

  const defaults = {
    fontFamily: (cascadeAttrs?.['style:font_family'] as string) ?? 'Pretendard',
    fontSizePt100: (cascadeAttrs?.['style:font_size'] as number) ?? 1200,
    lineHeight: (cascadeAttrs?.['paragraph:line_height'] as number) ?? 160,
    paragraphIndentPx: (((json.settings.paragraph_indent as number) ?? 100) / 100) * 16,
    blockGapPx: (((json.settings.block_gap as number) ?? 100) / 100) * 16,
  };

  // 에셋 로딩
  const imageIds = collectNodeIds(nodes, 'image');
  const embedIds = collectNodeIds(nodes, 'embed');
  const [images, embeds] = await Promise.all([loadImageAssets(imageIds), loadEmbeds(embedIds)]);

  return { nodes, settings: json.settings, defaults, images, embeds };
}
