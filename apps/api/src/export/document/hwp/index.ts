// spell-checker:words HWPUNIT
import { inArray } from 'drizzle-orm';
import { db, Embeds } from '@/db';
import { wasm } from '@/utils/wasm';
import { loadImageAssets } from '../external';
import { resolveFontEntry } from '../font';
import { buildBodyStream } from './body';
import { buildDocInfoStream, IdTable } from './doc-info';
import { collectBinDataStreams } from './image';
import { buildOle2 } from './ole2';
import { allocate, compressStream, pxToHwpunit } from './records';
import type { ImageAsset } from '../external';
import type { FontNameMap } from '../font';
import type { HwpConvertContext, NodeEntry } from './body';
import type { CharShapeEntry, DocInfoTables, ParaShapeEntry } from './doc-info';

type DocumentJson = {
  settings: Record<string, unknown>;
  nodes: Record<string, NodeEntry>;
};

export type GenerateDocumentHwpParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
  fontNameMap: FontNameMap;
};

export async function generateDocumentHwp(params: GenerateDocumentHwpParams): Promise<Uint8Array> {
  const { snapshot } = params;

  // 1. snapshotToJson
  const json = (await wasm.snapshotToJson(snapshot)) as unknown as DocumentJson;

  // 2. 이미지/임베드 로드
  const imageIds = collectNodeIds(json.nodes, 'image');
  const embedIds = collectNodeIds(json.nodes, 'embed');
  const [assets, embeds] = await Promise.all([loadImages(imageIds), loadEmbeds(embedIds)]);

  // 3. Root 노드에서 기본값 추출
  const rootId = Object.keys(json.nodes).find((id) => json.nodes[id].type === 'root');
  if (!rootId) throw new Error('Root node not found');

  const rootEntry = json.nodes[rootId];
  const cascadeAttrs = (rootEntry.cascade_attrs as Record<string, unknown>) ?? {};
  const defaultFont = (cascadeAttrs['style:font_family'] as string) ?? 'Pretendard';
  const defaultFontSizePt100 = (cascadeAttrs['style:font_size'] as number) ?? 1200;
  const defaultLineHeight = (cascadeAttrs['paragraph:line_height'] as number) ?? 160;

  const paragraphIndentRaw = (json.settings.paragraph_indent as number) ?? 100;
  const paragraphIndentPx = (paragraphIndentRaw / 100) * 16;
  // 한/글 paragraph spacing은 200 HWPUNIT/pt (dimension의 2배) — 레퍼런스 파일 분석으로 확인
  const paragraphIndentHwp = pxToHwpunit(paragraphIndentPx) * 2;

  const blockGapRaw = (json.settings.block_gap as number) ?? 100;
  const blockGapPx = (blockGapRaw / 100) * 16;
  const blockGapHwp = pxToHwpunit(blockGapPx) * 2;

  // 4. DocInfo 테이블 초기화 + 기본 항목 등록 (ID 0)
  const tables: DocInfoTables = {
    fonts: new IdTable(),
    charShapes: new IdTable(),
    paraShapes: new IdTable(),
    borderFills: new IdTable(),
    binData: new IdTable(),
    numberings: new IdTable(),
    bullets: new IdTable(),
  };

  const defaultFontEntry = resolveFontEntry(params.fontNameMap, defaultFont, 400);
  const defaultFontId = tables.fonts.intern(
    {
      name: defaultFontEntry?.faceName ?? defaultFont,
      postScriptName: defaultFontEntry?.faceDefault ?? defaultFont,
    },
    defaultFontEntry?.postScriptName ?? defaultFont,
  );

  const defaultCharShape: CharShapeEntry = {
    fontId: defaultFontId,
    baseSize: defaultFontSizePt100,
    bold: false,
    italic: false,
    underline: false,
    strikethrough: false,
    textColor: 0x00_00_00_00,
    underlineColor: 0x00_00_00_00,
    shadeColor: 0xff_ff_ff_ff,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: 0x00_00_00_00,
    letterSpacing: 0,
  };
  const defaultCharShapeKey = `${defaultFontId}:${defaultFontSizePt100}:false:false:false:false:0:4294967295:0`;
  const defaultCharShapeId = tables.charShapes.intern(defaultCharShape, defaultCharShapeKey);

  const lineSpacing = defaultLineHeight;
  const defaultParaShape: ParaShapeEntry = {
    alignment: 0,
    lineSpacingType: 0,
    lineSpacing,
    spaceBefore: 0,
    spaceAfter: blockGapHwp,
    indent: 0,
    leftMargin: 0,
    rightMargin: 0,
    headType: 0,
    headLevel: 0,
    numberingId: 0,
  };
  const defaultParaShapeKey = `0:${lineSpacing}:0:${blockGapHwp}:0:0:0:0`;
  const defaultParaShapeId = tables.paraShapes.intern(defaultParaShape, defaultParaShapeKey);

  // 기본 BORDER_FILL (빈 테두리, 흰 배경) — PAGE_BORDER_FILL 등이 참조
  tables.borderFills.intern(
    {
      leftType: 0,
      rightType: 0,
      topType: 0,
      bottomType: 0,
      leftWidth: 0,
      rightWidth: 0,
      topWidth: 0,
      bottomWidth: 0,
      leftColor: 0,
      rightColor: 0,
      topColor: 0,
      bottomColor: 0,
      fillType: 0,
      fillColor: 0,
    },
    'default-empty',
  );

  // 5. HwpConvertContext 구성
  const ctx: HwpConvertContext = {
    nodes: json.nodes,
    assets,
    embeds,
    tables,
    pageLayout: {
      pageWidth: params.pageWidth,
      pageHeight: params.pageHeight,
      pageMarginTop: params.pageMarginTop,
      pageMarginBottom: params.pageMarginBottom,
      pageMarginLeft: params.pageMarginLeft,
      pageMarginRight: params.pageMarginRight,
    },
    listStack: [],
    fontNameMap: params.fontNameMap,
    defaultFamilyName: defaultFont,
    defaultFontId,
    defaultCharShapeId,
    defaultParaShapeId,
    paragraphIndentHwp,
    blockGapHwp,
    defaultFontSizePt100,
    defaultLineHeight,
    instanceCounter: 0,
  };

  // 6. Pass 1+2: BodyText 생성 (내부에서 tables에 항목이 추가됨)
  const bodyStream = buildBodyStream(ctx);

  // 7. DocInfo 생성 (테이블이 확정된 후)
  const docInfoStream = buildDocInfoStream(tables);

  // 8. FileHeader 생성
  const fileHeader = buildFileHeader();

  // 9. BinData 스트림 수집
  const binDataStreams = collectBinDataStreams(tables, assets);

  // 10. OLE2 패키징
  return buildOle2([
    { path: 'FileHeader', data: fileHeader },
    { path: 'DocInfo', data: compressStream(docInfoStream) },
    { path: 'BodyText/Section0', data: compressStream(bodyStream) },
    ...[...binDataStreams].map(([path, data]) => ({ path, data: compressStream(data) })),
  ]);
}

function buildFileHeader(): Uint8Array {
  const { buf, view } = allocate(256);
  const sig = new TextEncoder().encode('HWP Document File');
  buf.set(sig, 0);
  view.setUint32(32, 0x05_01_01_00, true); // version 5.1.1.0
  view.setUint32(36, 0x00_00_00_01, true); // flags: compressed
  return buf;
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

async function loadImages(imageIds: string[]): Promise<Map<string, ImageAsset>> {
  if (imageIds.length === 0) return new Map();
  return loadImageAssets(imageIds);
}

async function loadEmbeds(ids: string[]): Promise<Map<string, { url: string; title: string | null }>> {
  if (ids.length === 0) return new Map();
  const rows = await db.select({ id: Embeds.id, url: Embeds.url, title: Embeds.title }).from(Embeds).where(inArray(Embeds.id, ids));
  return new Map(rows.map((r) => [r.id, { url: r.url, title: r.title }]));
}
