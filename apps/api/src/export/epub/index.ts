// cspell:ignore OEBPS
import { inArray } from 'drizzle-orm';
import JSZip from 'jszip';
import { db, Embeds } from '@/db';
import { wasm } from '@/utils/wasm';
import { loadImageAssets } from '../external';
import { collectUsedFonts, loadFontFiles } from './fonts';
import { generateContainerXml, generateContentOpf, generateNavXhtml } from './meta';
import { renderBodyHtml } from './nodes';
import { generateStylesheet } from './styles';
import { extFromFormat } from './utils';
import type { DocumentFontFamily } from '@/utils/document';
import type { ConvertContext } from './nodes';
import type { NodeEntry } from './types';

type DocumentJson = {
  settings: Record<string, unknown>;
  nodes: Record<string, NodeEntry>;
};

export type GenerateDocumentEpubParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  fontFamilies: DocumentFontFamily[];
};

export async function generateDocumentEpub(params: GenerateDocumentEpubParams): Promise<Uint8Array> {
  const { snapshot, title, author, fontFamilies } = params;

  const json = (await wasm.snapshotToJson(snapshot)) as unknown as DocumentJson;

  // 외부 에셋 로딩
  const imageIds = collectNodeIds(json.nodes, 'image');
  const embedIds = collectNodeIds(json.nodes, 'embed');
  const [assets, embeds] = await Promise.all([loadImageAssets(imageIds), loadEmbeds(embedIds)]);

  // 문서 기본 스타일 추출
  const rootId = Object.keys(json.nodes).find((id) => json.nodes[id].type === 'root');
  if (!rootId) {
    throw new Error('Root node not found in document');
  }

  const rootEntry = json.nodes[rootId];
  const cascadeAttrs = rootEntry.cascade_attrs as Record<string, unknown> | undefined;
  const defaultFont = (cascadeAttrs?.['style:font_family'] as string) ?? 'Pretendard';
  const defaultFontSizePt = ((cascadeAttrs?.['style:font_size'] as number) ?? 1200) / 100;
  const defaultLineHeight = (cascadeAttrs?.['paragraph:line_height'] as number) ?? 160;
  const paragraphIndentPx = (((json.settings.paragraph_indent as number) ?? 100) / 100) * 16;
  const blockGapPx = (((json.settings.block_gap as number) ?? 100) / 100) * 16;

  // 폰트 수집 및 로딩
  const usedFonts = collectUsedFonts(json.nodes, defaultFont);
  const fontFiles = await loadFontFiles(usedFonts, fontFamilies);

  // HTML 렌더링
  const ctx: ConvertContext = { nodes: json.nodes, assets, embeds };
  const bodyHtml = renderBodyHtml(rootEntry.children ?? [], ctx);

  // CSS 생성
  const stylesheet = generateStylesheet(fontFiles, {
    fontFamily: defaultFont,
    fontSizePt: defaultFontSizePt,
    lineHeight: defaultLineHeight,
    blockGapPx,
    paragraphIndentPx,
  });

  // EPUB 아카이브 패킹
  const documentXhtml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>${escapeXml(title)}</title>
  <link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
${bodyHtml}
</body>
</html>`;

  const imagesMeta = [...assets.entries()].map(([id, asset]) => {
    const ext = extFromFormat(asset.format);
    return { id, filename: `${id}.${ext}`, mediaType: asset.format, bytes: asset.bytes };
  });

  const fontsMeta = [...fontFiles.values()].map((f) => ({ filename: f.filename }));

  const zip = new JSZip();
  zip.file('mimetype', 'application/epub+zip', { compression: 'STORE' });
  zip.file('META-INF/container.xml', generateContainerXml());
  zip.file('OEBPS/content.opf', generateContentOpf({ title, author, images: imagesMeta, fonts: fontsMeta }));
  zip.file('OEBPS/nav.xhtml', generateNavXhtml(title));
  zip.file('OEBPS/style.css', stylesheet);
  zip.file('OEBPS/document.xhtml', documentXhtml);

  for (const img of imagesMeta) {
    zip.file(`OEBPS/images/${img.filename}`, img.bytes);
  }

  for (const font of fontFiles.values()) {
    zip.file(`OEBPS/fonts/${font.filename}`, font.bytes);
  }

  return zip.generateAsync({ type: 'uint8array' });
}

function collectNodeIds(nodes: Record<string, { type: string; id?: string }>, type: string): string[] {
  const ids: string[] = [];
  for (const entry of Object.values(nodes)) {
    if (entry.type === type && entry.id) {
      ids.push(entry.id);
    }
  }
  return ids;
}

async function loadEmbeds(ids: string[]): Promise<Map<string, { url: string; title: string | null }>> {
  if (ids.length === 0) return new Map();
  const rows = await db.select({ id: Embeds.id, url: Embeds.url, title: Embeds.title }).from(Embeds).where(inArray(Embeds.id, ids));
  return new Map(rows.map((r) => [r.id, { url: r.url, title: r.title }]));
}

function escapeXml(str: string): string {
  return str.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}
