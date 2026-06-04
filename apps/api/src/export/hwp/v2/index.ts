// spell-checker:words HWPUNIT
import { findFontFamily, nearestWeight } from '../../core/fonts.ts';
import { parseDocumentV2 } from '../../core/v2/document.ts';
import { buildDocInfoStream } from '../doc-info.ts';
import { collectBinDataStreams } from '../image.ts';
import { buildOle2 } from '../ole2.ts';
import { allocate, compressStream, IdTable, pxToHwpunit } from '../records.ts';
import { buildBodyStreamV2 } from './body.ts';
import type { ExportFontFamily, PageLayout } from '../../core/types.ts';
import type { CharShapeEntry, DocInfoTables, ParaShapeEntry } from '../doc-info.ts';
import type { HwpConvertContext } from '../types.ts';

export type GenerateDocumentHwpV2Params = {
  graph: Uint8Array;
  title: string;
  author: string;
  fonts: ExportFontFamily[];
  layout: PageLayout;
};

export async function generateDocumentHwpV2(params: GenerateDocumentHwpV2Params): Promise<Uint8Array> {
  const parsed = await parseDocumentV2(params.graph);
  const { defaults } = parsed;

  const paragraphIndentHwp = pxToHwpunit(defaults.paragraphIndentPx) * 2;
  const blockGapHwp = pxToHwpunit(defaults.blockGapPx) * 2;

  const tables: DocInfoTables = {
    fonts: new IdTable(),
    charShapes: new IdTable(),
    paraShapes: new IdTable(),
    borderFills: new IdTable(),
    binData: new IdTable(),
    numberings: new IdTable(),
    bullets: new IdTable(),
  };

  const defaultFam = findFontFamily(params.fonts, defaults.fontFamily);
  const defaultFontEntry = defaultFam ? nearestWeight(defaultFam.weights, 400) : undefined;
  const defaultFontId = tables.fonts.intern(
    {
      name: defaultFontEntry?.localizedName ?? defaultFontEntry?.name ?? defaults.fontFamily,
      postScriptName: defaultFontEntry?.name ?? defaults.fontFamily,
    },
    defaultFontEntry?.postScriptName ?? defaults.fontFamily,
  );

  const defaultCharShape: CharShapeEntry = {
    fontId: defaultFontId,
    baseSize: defaults.fontSizePt100,
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
  const defaultCharShapeKey = `${defaultFontId}:${defaults.fontSizePt100}:false:false:false:false:0:4294967295:0`;
  const defaultCharShapeId = tables.charShapes.intern(defaultCharShape, defaultCharShapeKey);

  const lineSpacing = defaults.lineHeight;
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

  const ctx: HwpConvertContext = {
    nodes: {},
    assets: parsed.images,
    embeds: parsed.embeds,
    tables,
    pageLayout: params.layout,
    listStack: [],
    fonts: params.fonts,
    sectionDefEmitted: false,
    defaultFamilyName: defaults.fontFamily,
    defaultFontId,
    defaultCharShapeId,
    defaultParaShapeId,
    paragraphIndentHwp,
    blockGapHwp,
    defaultFontSizePt100: defaults.fontSizePt100,
    defaultLineHeight: defaults.lineHeight,
    instanceCounter: 0,
  };

  const bodyStream = buildBodyStreamV2(parsed, ctx);
  const docInfoStream = buildDocInfoStream(tables);
  const fileHeader = buildFileHeader();
  const binDataStreams = collectBinDataStreams(tables, parsed.images);

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
