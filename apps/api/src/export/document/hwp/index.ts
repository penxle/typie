// spell-checker:words HWPUNIT ENDOFCHAIN
import CFB from 'cfb';
import { inArray } from 'drizzle-orm';
import { db, Embeds } from '@/db';
import { wasm } from '@/utils/wasm';
import { loadImageAssets } from '../external';
import { buildBodyStream } from './body';
import { buildDocInfoStream, IdTable } from './doc-info';
import { collectBinDataStreams } from './image';
import { allocate, compressStream, pxToHwpunit } from './records';
import type { ImageAsset } from '../external';
import type { FontNameEntry, FontNameMap, HwpConvertContext } from './body';
import type { CharShapeEntry, DocInfoTables, ParaShapeEntry } from './doc-info';

type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

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

  const defaultFontEntry = resolveEntry(params.fontNameMap, defaultFont, 400);
  const defaultFontFullName = defaultFontEntry?.fullName ?? defaultFont;
  const defaultFontId = tables.fonts.intern(
    { name: defaultFontFullName, postScriptName: defaultFontEntry?.postScriptName },
    defaultFontFullName,
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

  // 10. cfb로 OLE2 패키징
  const cfbContainer = CFB.utils.cfb_new();
  CFB.utils.cfb_del(cfbContainer, '/\u0001Sh33tJ5');
  CFB.utils.cfb_add(cfbContainer, '/FileHeader', fileHeader);
  CFB.utils.cfb_add(cfbContainer, '/DocInfo', compressStream(docInfoStream));
  CFB.utils.cfb_add(cfbContainer, '/BodyText/Section0', compressStream(bodyStream));
  for (const [path, data] of binDataStreams) {
    CFB.utils.cfb_add(cfbContainer, `/${path}`, compressStream(data));
  }

  const cfbBytes = new Uint8Array(CFB.write(cfbContainer, { type: 'array' }) as ArrayBuffer);
  fixCfbFat(cfbBytes);
  return cfbBytes;
}

/** cfb 패키지가 미사용 FAT 엔트리를 ENDOFCHAIN(0xFFFFFFFE)으로 기록하는 문제를 수정 → FREESECT(0xFFFFFFFF) */
function fixCfbFat(data: Uint8Array): void {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const sectorSize = 1 << view.getUint16(0x1e, true);
  const totalSectors = (data.byteLength - 512) / sectorSize;
  const fatSectorIndex = view.getUint32(0x4c, true);
  const fatOffset = 512 + fatSectorIndex * sectorSize;
  const entriesPerFat = sectorSize / 4;

  for (let i = totalSectors; i < entriesPerFat; i++) {
    view.setUint32(fatOffset + i * 4, 0xff_ff_ff_ff, true); // FREESECT
  }
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

/** fontNameMap에서 familyName + weight에 가장 가까운 엔트리를 찾는다 */
function resolveEntry(map: FontNameMap, familyName: string, weight: number): FontNameEntry | undefined {
  const entries = map.get(familyName);
  if (!entries || entries.length === 0) return undefined;
  let best = entries[0];
  let bestDist = Math.abs(best.weight - weight);
  for (let i = 1; i < entries.length; i++) {
    const dist = Math.abs(entries[i].weight - weight);
    if (dist < bestDist) {
      best = entries[i];
      bestDist = dist;
    }
  }
  return best;
}
