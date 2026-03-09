// spell-checker:words HWPTAG HWPUNIT secd pbfv tdut tudt DBEAFE DCFCE
import { resolveFontEntry } from '../font';
import { resolveColorToHex } from '../theme';
import { buildGsoCtrlHeader, makeImageRecords, mapFormat } from './image';
import { allocate, concat, ctrlId, hexToColorref, HWPTAG, makeRecord, pxToHwpunit } from './records';
import { makeTableRecords as buildTableCoreRecords } from './table';
import type { ImageAsset } from '../external';
import type { FontNameMap } from '../font';
import type { BorderFillEntry, CharShapeEntry, DocInfoTables, ParaShapeEntry } from './doc-info';
import type { CellMargins } from './table';

type Style =
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'underline' }
  | { type: 'strikethrough' }
  | { type: 'font_size'; size: number }
  | { type: 'font_family'; family: string }
  | { type: 'font_weight'; weight: number }
  | { type: 'text_color'; color: string }
  | { type: 'background_color'; color: string }
  | { type: 'letter_spacing'; spacing: number };

type Annotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

type TextSegment = {
  text: string;
  styles: Style[];
  annotations: Annotation[];
};

export type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

type PageLayout = {
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
};

export type HwpConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, { url: string; title: string | null }>;
  tables: DocInfoTables;
  pageLayout: PageLayout;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  fontNameMap: FontNameMap;
  defaultFamilyName: string;
  defaultFontId: number;
  defaultCharShapeId: number;
  defaultParaShapeId: number;
  paragraphIndentHwp: number;
  blockGapHwp: number;
  defaultFontSizePt100: number;
  defaultLineHeight: number;
  instanceCounter: number;
};

const BLACK_COLORREF = 0x00_00_00_00;

// --- 공통 헬퍼 ---

/** PARA_LINE_SEG (36바이트) 기본값 생성 — 한/글이 재계산하지만 초기 힌트 필요 */
function makeDefaultParaLineSeg(level: number): Uint8Array {
  const { buf, view } = allocate(36);
  view.setInt32(8, 1000, true); // 줄의 높이
  view.setInt32(12, 1000, true); // 텍스트 부분의 높이
  view.setInt32(16, 850, true); // 베이스라인까지 거리
  view.setInt32(20, 600, true); // 줄간격
  view.setUint32(32, 0x00_06_00_00, true); // 태그: 첫+마지막 세그먼트
  return makeRecord(HWPTAG.PARA_LINE_SEG, level, buf);
}

/** 노드의 자식 중 paragraph 타입만 수집하여 인라인 세그먼트로 변환 */
function collectParagraphsFromChildren(
  entry: NodeEntry,
  ctx: HwpConvertContext,
): { segments: InlineSegment[]; align?: string; lineHeight?: number }[] {
  const paragraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[] = [];
  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry || childEntry.type !== 'paragraph') continue;
    paragraphs.push({
      segments: collectInlineSegments(childEntry, ctx),
      align: childEntry.align as string | undefined,
      lineHeight: childEntry.line_height as number | undefined,
    });
  }
  return paragraphs;
}

// --- 텍스트 세그먼트 → CharShape ID ---

function resolveCharShape(styles: Style[], ctx: HwpConvertContext): number {
  let familyName: string | undefined;
  let weight = 400;
  let hasExplicitWeight = false;
  let baseSize = ctx.defaultFontSizePt100;
  let bold = false;
  let italic = false;
  let underline = false;
  let strikethrough = false;
  let textColor = BLACK_COLORREF;
  let shadeColor = 0xff_ff_ff_ff;
  let letterSpacing = 0;

  for (const style of styles) {
    switch (style.type) {
      case 'bold': {
        bold = true;
        if (!hasExplicitWeight) weight = 700;
        break;
      }
      case 'italic': {
        italic = true;
        break;
      }
      case 'underline': {
        underline = true;
        break;
      }
      case 'strikethrough': {
        strikethrough = true;
        break;
      }
      case 'font_size': {
        baseSize = style.size;
        break;
      }
      case 'font_family': {
        familyName = style.family;
        break;
      }
      case 'font_weight': {
        weight = style.weight;
        hasExplicitWeight = true;
        bold = style.weight >= 600;
        break;
      }
      case 'text_color': {
        const hex = resolveColorToHex(`text.${style.color}`);
        if (hex) textColor = hexToColorref(hex);
        break;
      }
      case 'background_color': {
        const hex = resolveColorToHex(`bg.${style.color}`);
        if (hex) shadeColor = hexToColorref(hex);
        break;
      }
      case 'letter_spacing': {
        // em × 100 → HWP 자간 (-50 ~ +50 %)
        letterSpacing = Math.round(style.spacing);
        break;
      }
    }
  }

  // family + weight → font entry 조회 (한/글은 폰트 이름으로 weight를 매칭)
  const family = familyName ?? ctx.defaultFamilyName;
  const resolved = resolveFontEntry(ctx.fontNameMap, family, weight);
  const fontId = ctx.tables.fonts.intern(
    { name: resolved?.faceName ?? family, postScriptName: resolved?.faceDefault ?? family },
    resolved?.postScriptName ?? family,
  );

  const entry: CharShapeEntry = {
    fontId,
    baseSize,
    bold,
    italic,
    underline,
    strikethrough,
    textColor,
    underlineColor: textColor,
    shadeColor,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: textColor,
    letterSpacing,
  };
  const key = `${fontId}:${baseSize}:${bold}:${italic}:${underline}:${strikethrough}:${textColor}:${shadeColor}:${letterSpacing}`;
  return ctx.tables.charShapes.intern(entry, key);
}

function mapAlignment(align: string): number {
  switch (align) {
    case 'justify': {
      return 0;
    }
    case 'left': {
      return 1;
    }
    case 'right': {
      return 2;
    }
    case 'center': {
      return 3;
    }
    default: {
      return 0;
    }
  }
}

/** 텍스트 너비를 HWPUNIT 단위로 추정 (가장 긴 줄 기준) */
function estimateTextWidthHwp(paragraphs: { segments: InlineSegment[] }[], fontSizeHwp: number): number {
  let maxLineWidth = 0;
  for (const p of paragraphs) {
    let lineWidth = 0;
    for (const seg of p.segments) {
      for (const ch of seg.text) {
        if (ch === '\n') {
          maxLineWidth = Math.max(maxLineWidth, lineWidth);
          lineWidth = 0;
        } else {
          const code = ch.codePointAt(0) ?? 0;
          // CJK / 한글 / 전각 문자: 1em, 그 외: 0.5em
          const isCjk =
            (code >= 0x30_00 && code <= 0x9f_ff) ||
            (code >= 0xac_00 && code <= 0xd7_af) ||
            (code >= 0xf9_00 && code <= 0xfa_ff) ||
            (code >= 0xff_00 && code <= 0xff_ef);
          lineWidth += isCjk ? fontSizeHwp : Math.floor(fontSizeHwp * 0.55);
        }
      }
    }
    maxLineWidth = Math.max(maxLineWidth, lineWidth);
  }
  return maxLineWidth;
}

function resolveParaShape(
  ctx: HwpConvertContext,
  opts: {
    align?: string;
    lineHeight?: number;
    indent?: number;
    spaceBefore?: number;
    spaceAfter?: number;
    headType?: number;
    headLevel?: number;
    numberingId?: number;
  },
): number {
  const alignment = opts.align ? mapAlignment(opts.align) : 0;
  const lineHeight = opts.lineHeight ?? ctx.defaultLineHeight;
  // lineSpacingType 0 = 글자에 대한 비율(%), 160 = 160%
  const lineSpacing = lineHeight;
  const entry: ParaShapeEntry = {
    alignment,
    lineSpacingType: 0, // 글자에 따라(%)
    lineSpacing,
    spaceBefore: opts.spaceBefore ?? 0,
    spaceAfter: opts.spaceAfter ?? ctx.blockGapHwp,
    indent: opts.indent ?? 0,
    leftMargin: 0,
    rightMargin: 0,
    headType: opts.headType ?? 0,
    headLevel: opts.headLevel ?? 0,
    numberingId: opts.numberingId ?? 0,
  };
  const key = `${alignment}:${lineSpacing}:${entry.spaceBefore}:${entry.spaceAfter}:${entry.indent}:${entry.headType}:${entry.headLevel}:${entry.numberingId}`;
  return ctx.tables.paraShapes.intern(entry, key);
}

// --- 구역 정의 ---

function buildSectionDef(ctx: HwpConvertContext): Uint8Array[] {
  const records: Uint8Array[] = [];
  const pl = ctx.pageLayout;

  // CTRL_HEADER (ctrl_id: "secd") + 구역 정의 속성 (표 129, 26바이트)
  const { buf: ctrlBuf, view: ctrlView } = allocate(4 + 26);
  ctrlView.setUint32(0, ctrlId('secd'), true);
  ctrlView.setUint32(14, 8000, true); // 기본 탭 간격 (8000 HWPUNIT, 한/글 기본값)
  records.push(makeRecord(HWPTAG.CTRL_HEADER, 1, ctrlBuf));

  // PAGE_DEF (표 131, 40바이트)
  const { buf: pageBuf, view: pageView } = allocate(40);
  pageView.setUint32(0, pxToHwpunit(pl.pageWidth), true);
  pageView.setUint32(4, pxToHwpunit(pl.pageHeight), true);
  pageView.setUint32(8, pxToHwpunit(pl.pageMarginLeft), true);
  pageView.setUint32(12, pxToHwpunit(pl.pageMarginRight), true);
  pageView.setUint32(16, pxToHwpunit(pl.pageMarginTop), true);
  pageView.setUint32(20, pxToHwpunit(pl.pageMarginBottom), true);
  records.push(makeRecord(HWPTAG.PAGE_DEF, 2, pageBuf));

  // FOOTNOTE_SHAPE — 각주 (표 133, 28바이트, 한/글 기본값)
  const { buf: fn1, view: fn1v } = allocate(28);
  fn1v.setUint16(8, 0x00_29, true); // 뒤 장식 문자 = ')'
  fn1v.setUint16(10, 1, true); // 시작 번호 = 1
  fn1v.setUint32(12, 0xff_ff_ff_ff, true); // 구분선 길이 = -1 (전체)
  fn1v.setUint16(16, 850, true); // 구분선 위 여백
  fn1v.setUint16(18, 567, true); // 구분선 아래 여백
  fn1v.setUint16(20, 283, true); // 주석 사이 여백
  fn1v.setUint8(22, 1); // 구분선 종류 = 실선
  fn1v.setUint8(23, 1); // 구분선 굵기
  records.push(makeRecord(HWPTAG.FOOTNOTE_SHAPE, 2, fn1));

  // FOOTNOTE_SHAPE — 미주 (표 133, 28바이트, 한/글 기본값)
  const { buf: fn2, view: fn2v } = allocate(28);
  fn2v.setUint16(8, 0x00_29, true);
  fn2v.setUint16(10, 1, true);
  fn2v.setUint32(12, 0x00_e0_2f_f8, true);
  fn2v.setUint16(16, 850, true);
  fn2v.setUint16(18, 567, true);
  fn2v.setUint8(22, 1);
  fn2v.setUint8(23, 1);
  records.push(makeRecord(HWPTAG.FOOTNOTE_SHAPE, 2, fn2));

  // PAGE_BORDER_FILL (표 135, 14바이트) × 3 (양쪽/홀수/짝수, 한/글 기본값)
  for (let i = 0; i < 3; i++) {
    const { buf: pbf, view: pbfv } = allocate(14);
    pbfv.setUint16(0, 1, true); // 속성
    pbfv.setUint16(4, 1417, true); // 왼쪽 여백
    pbfv.setUint16(6, 1417, true); // 오른쪽 여백
    pbfv.setUint16(8, 1417, true); // 위 여백
    pbfv.setUint16(10, 1417, true); // 아래 여백
    pbfv.setUint16(12, 1, true);
    records.push(makeRecord(HWPTAG.PAGE_BORDER_FILL, 2, pbf));
  }

  // CTRL_HEADER (ctrl_id: "cold") — 단 정의 (표 138, 12바이트)
  const { buf: coldBuf, view: coldView } = allocate(4 + 12);
  coldView.setUint32(0, ctrlId('cold'), true);
  coldView.setUint16(4, 0x10_04, true); // 단종류=일반, 단개수=1, 단너비동일=1
  records.push(makeRecord(HWPTAG.CTRL_HEADER, 1, coldBuf));

  return records;
}

// --- 문단 생성 ---

type InlineSegment = {
  text: string;
  charShapeId: number;
  link?: string;
  ruby?: string;
  rubyCharShapeId?: number;
};

function collectInlineSegments(entry: NodeEntry, ctx: HwpConvertContext): InlineSegment[] {
  const segments: InlineSegment[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'text') {
      const textSegments = (childEntry.text as TextSegment[]) ?? [];
      for (const seg of textSegments) {
        const charShapeId = resolveCharShape(seg.styles ?? [], ctx);
        const rubyAnnotation = seg.annotations?.find((a): a is Extract<Annotation, { type: 'ruby' }> => a.type === 'ruby');
        const rubyCharShapeId = rubyAnnotation ? resolveRubyCharShape(charShapeId, ctx) : undefined;
        segments.push({ text: seg.text, charShapeId, ruby: rubyAnnotation?.text, rubyCharShapeId });
      }
    } else if (childEntry.type === 'hard_break') {
      segments.push({ text: '\n', charShapeId: ctx.defaultCharShapeId });
    }
  }

  return segments;
}

function makeParagraph(
  segments: InlineSegment[],
  paraShapeId: number,
  defaultCharShapeId: number,
  level: number,
  extraCtrlRecords?: Uint8Array[],
): Uint8Array[] {
  const records: Uint8Array[] = [];
  const hasSectionDef = extraCtrlRecords && extraCtrlRecords.length > 0;

  // PARA_TEXT 구성: UTF-16LE + 문단 끝 char 13
  const textParts: number[] = [];
  const charShapePairs: { pos: number; id: number }[] = [];
  let currentPos = 0;

  // 구역/단 정의: secd(8 WCHAR) + cold(8 WCHAR) 삽입
  if (hasSectionDef) {
    charShapePairs.push({ pos: currentPos, id: defaultCharShapeId });
    // secd: char 2 + ctrl_id(2 WCHAR) + padding(4 WCHAR) + char 2 = 8 WCHAR
    const secdId = ctrlId('secd');
    textParts.push(2, secdId & 0xff_ff, (secdId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    // cold: char 2 + ctrl_id(2 WCHAR) + padding(4 WCHAR) + char 2 = 8 WCHAR
    const coldId = ctrlId('cold');
    textParts.push(2, coldId & 0xff_ff, (coldId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    currentPos += 16; // 8 + 8 WCHAR
  }

  const linkCtrlRecords: Uint8Array[] = [];
  const rubyCtrlRecords: Uint8Array[] = [];
  let hasFieldControl = false;
  let hasRubyControl = false;

  for (const seg of segments) {
    if (seg.text.length === 0) continue;

    if (seg.ruby) {
      // Ruby (덧말): CHAR23 extended control (8 WCHAR) + CTRL_HEADER "tdut"
      hasRubyControl = true;
      charShapePairs.push({ pos: currentPos, id: seg.charShapeId });
      const tudtId = ctrlId('tdut');
      textParts.push(23, tudtId & 0xff_ff, (tudtId >> 16) & 0xff_ff, 0, 0, 0, 0, 23);
      currentPos += 8;

      rubyCtrlRecords.push(buildRubyCtrlHeader(seg.text, seg.ruby, seg.rubyCharShapeId ?? 0, level + 1));
      continue;
    }

    if (seg.link) {
      hasFieldControl = true;
      // 필드 시작 전 charShape가 없으면 기본값 추가
      if (charShapePairs.length === 0) {
        charShapePairs.push({ pos: currentPos, id: defaultCharShapeId });
      }
      // FIELD_BEGIN (char 3, 8 WCHAR): char3 + ctrlId(2W) + params(4W) + char3
      const hlkId = ctrlId('%hlk');
      textParts.push(3, hlkId & 0xff_ff, (hlkId >> 16) & 0xff_ff, 0, 0, 0, 0, 3);
      currentPos += 8;
    }

    charShapePairs.push({ pos: currentPos, id: seg.charShapeId });

    for (const ch of seg.text) {
      if (ch === '\n') {
        textParts.push(10); // line break
        currentPos += 1;
      } else {
        textParts.push(ch.codePointAt(0) ?? 0);
        currentPos += 1;
      }
    }

    if (seg.link) {
      // FIELD_SEP (char 4)
      textParts.push(4);
      currentPos += 1;
      // 필드 데이터 (6 WCHAR: ctrlId without '%' + padding)
      const hlkId = ctrlId('%hlk');
      const hlkIdNoPercent = hlkId & 0x00_ff_ff_ff;
      textParts.push(hlkIdNoPercent & 0xff_ff, (hlkIdNoPercent >> 16) & 0xff_ff, 0, 0, 0, 0);
      currentPos += 6;
      // FIELD_END (char 4)
      textParts.push(4);
      currentPos += 1;
      // %hlk CTRL_HEADER 레코드 생성 (fieldId는 PARA_TEXT 위치 기반으로 유일성 보장)
      linkCtrlRecords.push(buildHyperlinkCtrlHeader(seg.link, level + 1, currentPos));
    }
  }

  // 문단 끝 (char 13) — nchars에는 포함되지만 PARA_TEXT에는 빈 단락일 때 제외
  const isEmpty = currentPos === 0;
  textParts.push(13);
  currentPos += 1;

  // 빈 문단인 경우 기본 charShape 추가
  if (charShapePairs.length === 0) {
    charShapePairs.push({ pos: 0, id: defaultCharShapeId });
  }

  // 중복 charShape 합치기
  const mergedPairs: { pos: number; id: number }[] = [];
  for (const pair of charShapePairs) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    if (mergedPairs.length > 0 && mergedPairs.at(-1)!.id === pair.id) continue;
    mergedPairs.push(pair);
  }

  // PARA_HEADER (24바이트) — 표 58
  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, currentPos, true); // char_count (nchars)
  // control_mask: bit 2 = 구역/단 정의(char 2), bit 3 = 필드 시작(char 3), bit 4 = 필드 끝(char 4)
  let controlMask = 0;
  if (hasSectionDef) controlMask |= 0x04;
  if (hasFieldControl) controlMask |= 0x18; // bit 3 + bit 4
  if (hasRubyControl) controlMask |= 0x80_00_00; // bit 23 = char 23 (ruby "tdut")
  headerView.setUint32(4, controlMask, true);
  headerView.setUint16(8, paraShapeId, true); // para_shape_id
  headerView.setUint8(10, 0); // para_style_id
  headerView.setUint8(11, hasSectionDef ? 0x03 : 0); // column_break_type: 구역(0x01)+다단(0x02)
  headerView.setUint16(12, mergedPairs.length, true); // char_shape_count
  headerView.setUint16(14, 0, true); // range_tag_count
  headerView.setUint16(16, 1, true); // nAlignInfo = 1 (PARA_LINE_SEG 1개)
  headerView.setUint32(18, 0, true); // instance_id
  headerView.setUint16(22, 0, true); // merge_para
  records.push(makeRecord(HWPTAG.PARA_HEADER, level, headerBuf));

  // PARA_TEXT — 빈 단락(nchars=1, CR만)에서는 PARA_TEXT를 생략 (HWP 규격)
  if (!isEmpty) {
    const textBuf = new Uint8Array(textParts.length * 2);
    const textView = new DataView(textBuf.buffer);
    for (const [i, textPart] of textParts.entries()) {
      textView.setUint16(i * 2, textPart, true);
    }
    records.push(makeRecord(HWPTAG.PARA_TEXT, level + 1, textBuf));
  }

  // PARA_CHAR_SHAPE
  const csSize = mergedPairs.length * 8;
  const { buf: csBuf, view: csView } = allocate(csSize);
  for (const [i, mergedPair] of mergedPairs.entries()) {
    csView.setUint32(i * 8, mergedPair.pos, true);
    csView.setUint32(i * 8 + 4, mergedPair.id, true);
  }
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, level + 1, csBuf), makeDefaultParaLineSeg(level + 1));

  // 확장 컨트롤 레코드 (구역 정의 등)
  if (extraCtrlRecords) {
    records.push(...extraCtrlRecords);
  }

  // Ruby(덧말) CTRL_HEADER 레코드
  if (rubyCtrlRecords.length > 0) {
    records.push(...rubyCtrlRecords);
  }

  // 하이퍼링크 CTRL_HEADER 레코드
  if (linkCtrlRecords.length > 0) {
    records.push(...linkCtrlRecords);
  }

  return records;
}

function makeEmptyParagraph(paraShapeId: number, charShapeId: number, level: number): Uint8Array[] {
  return makeParagraph([], paraShapeId, charShapeId, level);
}

/** FIELD_HYPERLINK(%hlk) CTRL_HEADER 레코드 생성 */
function buildHyperlinkCtrlHeader(url: string, level: number, fieldId: number): Uint8Array {
  // URL 내 콜론을 이스케이프: : → \:
  const escapedUrl = url.replaceAll(':', String.raw`\:`);
  // 커맨드 형식: URL;openFrame;addToHistory;reserved;
  const command = `${escapedUrl};1;0;0;`;
  const cmdLen = command.length;

  // ctrl_id(4) + attr(4) + etc_byte(1) + cmd_len(2) + cmd(2×len) + id(4) + trailing(4)
  const totalSize = 4 + 4 + 1 + 2 + cmdLen * 2 + 4 + 4;
  const { buf, view } = allocate(totalSize);

  let offset = 0;
  view.setUint32(offset, ctrlId('%hlk'), true);
  offset += 4;
  view.setUint32(offset, 0x00_00_a8_00, true); // attr: 링크 생성 + 미방문 + 내용 수정됨
  offset += 4;
  buf[offset] = 0; // etc_byte
  offset += 1;
  view.setUint16(offset, cmdLen, true);
  offset += 2;
  for (let i = 0; i < cmdLen; i++) {
    view.setUint16(offset, command.codePointAt(i) ?? 0, true);
    offset += 2;
  }
  view.setUint32(offset, fieldId, true);
  offset += 4;
  // trailing zeros (4 bytes)

  return makeRecord(HWPTAG.CTRL_HEADER, level, buf);
}

/** Ruby annotation charShape: base charShape의 50% 크기 */
function resolveRubyCharShape(baseCharShapeId: number, ctx: HwpConvertContext): number {
  const allCharShapes = ctx.tables.charShapes.getAll() as CharShapeEntry[];
  const baseEntry = allCharShapes[baseCharShapeId];
  const baseSize = baseEntry?.baseSize ?? ctx.defaultFontSizePt100;
  const fontId = baseEntry?.fontId ?? ctx.defaultFontId;
  const rubySize = Math.round(baseSize * 0.5);
  const entry: CharShapeEntry = {
    fontId,
    baseSize: rubySize,
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
  return ctx.tables.charShapes.intern(entry, `ruby:${fontId}:${rubySize}`);
}

/** Ruby(덧말, "tdut") CTRL_HEADER 레코드 생성 */
function buildRubyCtrlHeader(baseText: string, rubyText: string, rubyCharShapeId: number, level: number): Uint8Array {
  // ctrlId(4) + str[0..9](count-prefixed UTF-16) + trailing charShapeId(4)
  const str0Bytes = baseText.length * 2;
  const str1Bytes = rubyText.length * 2;
  const totalSize = 4 + (2 + str0Bytes) + (2 + str1Bytes) + 8 * 2 + 4;
  const { buf, view } = allocate(totalSize);

  let offset = 0;
  view.setUint32(offset, ctrlId('tdut'), true);
  offset += 4;

  // str[0]: base text
  view.setUint16(offset, baseText.length, true);
  offset += 2;
  for (let i = 0; i < baseText.length; i++) {
    view.setUint16(offset, baseText.codePointAt(i) ?? 0, true);
    offset += 2;
  }

  // str[1]: ruby annotation text
  view.setUint16(offset, rubyText.length, true);
  offset += 2;
  for (let i = 0; i < rubyText.length; i++) {
    view.setUint16(offset, rubyText.codePointAt(i) ?? 0, true);
    offset += 2;
  }

  // str[2]-str[9]: empty strings (8 × uint16(0))
  for (let i = 0; i < 8; i++) {
    view.setUint16(offset, 0, true);
    offset += 2;
  }

  // trailing: ruby annotation charShapeId
  view.setUint32(offset, rubyCharShapeId, true);

  return makeRecord(HWPTAG.CTRL_HEADER, level, buf);
}

function makePageBreakParagraph(paraShapeId: number, charShapeId: number, level: number): Uint8Array[] {
  const records: Uint8Array[] = [];

  // PARA_HEADER with column_break_type = 0x04 (page break)
  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, 1, true); // char_count = 1 (para end only)
  headerView.setUint16(8, paraShapeId, true);
  headerView.setUint8(11, 0x04); // page break
  headerView.setUint16(12, 1, true); // char_shape_count
  headerView.setUint16(16, 1, true); // nAlignInfo = 1
  records.push(makeRecord(HWPTAG.PARA_HEADER, level, headerBuf));

  // 빈 단락이므로 PARA_TEXT 생략 (HWP 규격: nchars=1일 때 PARA_TEXT 없음)

  // PARA_CHAR_SHAPE
  const { buf: csBuf, view: csView } = allocate(8);
  csView.setUint32(0, 0, true);
  csView.setUint32(4, charShapeId, true);
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, level + 1, csBuf), makeDefaultParaLineSeg(level + 1));

  return records;
}

// --- 인라인 개체 문단 헬퍼 ---

/** char 11 (그리기 개체) 1개를 포함하는 인라인 개체 문단 레코드 생성 */
function makeInlineObjectParagraph(
  ctx: HwpConvertContext,
  level: number,
  ctrlIdStr: string,
  opts?: { sectionRecords?: Uint8Array[]; paraShapeId?: number },
): Uint8Array[] {
  const records: Uint8Array[] = [];
  const sectionRecords = opts?.sectionRecords;
  const hasSectionDef = sectionRecords && sectionRecords.length > 0;

  const textParts: number[] = [];

  // 구역/단 정의 삽입 (secd + cold)
  if (hasSectionDef) {
    const secdId = ctrlId('secd');
    textParts.push(2, secdId & 0xff_ff, (secdId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    const coldId = ctrlId('cold');
    textParts.push(2, coldId & 0xff_ff, (coldId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
  }

  // 그리기 개체 컨트롤
  const id = ctrlId(ctrlIdStr);
  textParts.push(11, id & 0xff_ff, (id >> 16) & 0xff_ff, 0, 0, 0, 0, 11, 13);

  const nchars = textParts.length;

  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, nchars, true);
  headerView.setUint32(4, 0x08_00, true); // control_mask: bit 11 = 그리기 개체/표
  headerView.setUint16(8, opts?.paraShapeId ?? ctx.defaultParaShapeId, true);
  headerView.setUint16(12, 1, true); // char_shape_count
  headerView.setUint16(16, 1, true); // nAlignInfo = 1
  records.push(makeRecord(HWPTAG.PARA_HEADER, level, headerBuf));

  const textBuf = new Uint8Array(nchars * 2);
  const textView = new DataView(textBuf.buffer);
  for (let i = 0; i < nchars; i++) {
    textView.setUint16(i * 2, textParts[i], true);
  }
  records.push(makeRecord(HWPTAG.PARA_TEXT, level + 1, textBuf));

  const { buf: csBuf, view: csView } = allocate(8);
  csView.setUint32(4, ctx.defaultCharShapeId, true);
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, level + 1, csBuf), makeDefaultParaLineSeg(level + 1));

  // 구역/단 정의 레코드 추가
  if (hasSectionDef) {
    records.push(...sectionRecords);
  }

  return records;
}

// --- 수평선 ---

function makeHorizontalRule(ctx: HwpConvertContext): Uint8Array[] {
  const contentWidth = pxToHwpunit(ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight);
  // 컨트롤 높이: 줄 중앙 배치를 위해 fontSize × (2 - lineSpacing/100) 사용
  // 160%/12pt → 1200 × 0.4 = 480 hwpunit (≈4.8pt)
  const height = Math.max(pxToHwpunit(2), Math.round(ctx.defaultFontSizePt100 * (2 - ctx.defaultLineHeight / 100)));
  const instanceId = ++ctx.instanceCounter;

  const records: Uint8Array[] = [];

  // HR 전용 문단 생성 (PARA_LINE_SEG를 HR 높이에 맞춤)
  const ctrlIdVal = ctrlId('gso ');
  const textParts = [11, ctrlIdVal & 0xff_ff, (ctrlIdVal >> 16) & 0xff_ff, 0, 0, 0, 0, 11, 13];
  const nchars = textParts.length;

  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, nchars, true);
  headerView.setUint32(4, 0x08_00, true);
  headerView.setUint16(8, ctx.defaultParaShapeId, true);
  headerView.setUint16(12, 1, true);
  headerView.setUint16(16, 1, true);
  records.push(makeRecord(HWPTAG.PARA_HEADER, 0, headerBuf));

  const textBuf = new Uint8Array(nchars * 2);
  const textView = new DataView(textBuf.buffer);
  for (let i = 0; i < nchars; i++) {
    textView.setUint16(i * 2, textParts[i], true);
  }
  records.push(makeRecord(HWPTAG.PARA_TEXT, 1, textBuf));

  // HR 문단 CharShape: 기본 글꼴 크기 사용 → 위쪽 문단과 동일한 줄 높이(line-height trailing)로 위아래 여백 대칭

  const { buf: csBuf, view: csView } = allocate(8);
  csView.setUint32(4, ctx.defaultCharShapeId, true);
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, 1, csBuf));

  // PARA_LINE_SEG (한/글이 재계산하지만 초기 힌트 제공)
  const { buf: lsBuf, view: lsView } = allocate(36);
  lsView.setInt32(8, height, true);
  lsView.setInt32(12, height, true);
  lsView.setInt32(16, height, true);
  lsView.setInt32(20, height, true);
  lsView.setUint32(32, 0x00_06_00_00, true);
  records.push(
    makeRecord(HWPTAG.PARA_LINE_SEG, 1, lsBuf),
    makeRecord(HWPTAG.CTRL_HEADER, 1, buildGsoCtrlHeader(contentWidth, height, instanceId)),
    makeRecord(HWPTAG.SHAPE_COMPONENT, 2, buildLineShapeComponent(contentWidth, height)),
  );

  // SHAPE_COMPONENT_LINE (표 92, 20바이트: startXY(8) + endXY(8) + attr(4))
  const { buf: lineBuf, view: lineView } = allocate(20);
  lineView.setInt32(0, 0, true); // start_x
  lineView.setInt32(4, 0, true); // start_y
  lineView.setInt32(8, contentWidth, true); // end_x
  lineView.setInt32(12, 0, true); // end_y
  lineView.setUint32(16, 0, true); // attr
  records.push(makeRecord(HWPTAG.SHAPE_COMPONENT_LINE, 3, lineBuf));

  return records;
}

/** SHAPE_COMPONENT for line (표 82 + 83) */
function buildLineShapeComponent(width: number, height: number): Uint8Array {
  const renderingInfo = buildRenderingInfo();
  // 8(ctrl_ids) + 42(개체요소속성) + 146(rendering) + 11(border) + 8(fill) + 24(textbox+shadow) = 239
  const totalSize = 8 + 42 + renderingInfo.byteLength + 11 + 8 + 24;
  const { buf, view } = allocate(totalSize);
  let offset = 0;

  view.setUint32(0, ctrlId('$lin'), true);
  view.setUint32(4, ctrlId('$lin'), true);
  offset = 8;

  // x_offset(4) + y_offset(4) + group_level(2) + local_version(2)
  view.setUint16(offset + 10, 1, true); // local_version = 1
  offset += 12;
  view.setUint32(offset, width, true); // width_org
  offset += 4;
  view.setUint32(offset, height, true); // height_org
  offset += 4;
  view.setUint32(offset, width, true); // width_cur
  offset += 4;
  view.setUint32(offset, height, true); // height_cur
  offset += 4;
  // flags(4)
  offset += 4;
  // rotation(2)
  offset += 2;
  // center_x(4) + center_y(4)
  view.setInt32(offset, Math.floor(width / 2), true);
  offset += 4;
  view.setInt32(offset, Math.floor(height / 2), true);
  offset += 4;

  buf.set(renderingInfo, offset);
  offset += renderingInfo.byteLength;

  // 테두리 선 정보 (표 86, 11바이트)
  view.setUint32(offset, hexToColorref('000000'), true); // 선 색상
  offset += 4;
  view.setInt16(offset, 100, true); // 선 굵기
  offset += 2;
  view.setUint32(offset, 0x00_41_00_00, true); // 속성 (레퍼런스 값)
  offset += 4;
  offset += 1; // outline_style = 0

  // 채우기 정보 (표 28, type=0 → 8바이트)
  view.setUint32(offset, 0, true); // type = 0 (채우기 없음)
  offset += 4;
  view.setUint32(offset, 0, true); // 추가 채우기 속성 길이 = 0
  offset += 4;

  // textbox + shadow 영역 (24바이트, 모두 0)
  offset += 24;

  return buf;
}

/** Rendering 정보: identity matrix (cnt=1) */
function buildRenderingInfo(): Uint8Array {
  const { buf, view } = allocate(146);
  view.setUint16(0, 1, true);
  const identity = [1, 0, 0, 0, 1, 0];
  for (let m = 0; m < 3; m++) {
    for (let i = 0; i < 6; i++) {
      view.setFloat64(2 + m * 48 + i * 8, identity[i], true);
    }
  }
  return buf;
}

// --- 노드 변환 ---

function convertNode(nodeId: string, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const entry = ctx.nodes[nodeId];
  if (!entry) return [];

  switch (entry.type) {
    case 'paragraph': {
      const segments = collectInlineSegments(entry, ctx);
      const indent = ctx.paragraphIndentHwp;
      const paraShapeId = resolveParaShape(ctx, {
        align: entry.align as string | undefined,
        lineHeight: entry.line_height as number | undefined,
        indent,
      });

      if (isFirst) {
        const sectionRecords = buildSectionDef(ctx);
        return makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
      }
      return makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 0);
    }

    case 'blockquote': {
      return convertBlockquoteNode(entry, ctx, isFirst);
    }

    case 'callout': {
      return convertCalloutNode(entry, ctx, isFirst);
    }

    case 'horizontal_rule': {
      if (isFirst) {
        const sectionRecords = buildSectionDef(ctx);
        const emptyPara = makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
        return [...emptyPara, ...makeHorizontalRule(ctx)];
      }
      return makeHorizontalRule(ctx);
    }

    case 'page_break': {
      if (isFirst) {
        const sectionRecords = buildSectionDef(ctx);
        return makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
      }
      return makePageBreakParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0);
    }

    case 'bullet_list': {
      ctx.listStack.push({ type: 'bullet', depth: ctx.listStack.length });
      const items = convertChildren(entry, ctx, isFirst);
      ctx.listStack.pop();
      return items;
    }

    case 'ordered_list': {
      ctx.listStack.push({ type: 'ordered', depth: ctx.listStack.length });
      const items = convertChildren(entry, ctx, isFirst);
      ctx.listStack.pop();
      return items;
    }

    case 'list_item': {
      return convertListItem(entry, ctx, isFirst);
    }

    case 'table': {
      return convertTableNode(entry, ctx, isFirst);
    }

    case 'fold': {
      return convertFoldNode(entry, ctx, isFirst);
    }

    case 'image': {
      return convertImageNode(entry, ctx, isFirst);
    }

    case 'embed': {
      return convertEmbedNode(entry, ctx, isFirst);
    }

    case 'file':
    case 'archived': {
      const label = entry.type === 'file' ? '[파일]' : '[보관된 블록]';
      return convertPlaceholderNode(label, ctx, isFirst);
    }

    default: {
      return [];
    }
  }
}

function convertChildren(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const results: Uint8Array[] = [];
  let first = isFirst;
  for (const childId of entry.children ?? []) {
    results.push(...convertNode(childId, ctx, first));
    first = false;
  }
  return results;
}

// --- 목록 아이템 ---

function convertListItem(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const currentList = ctx.listStack.at(-1);
  const listType = currentList?.type ?? 'bullet';
  const level = currentList?.depth ?? 0;
  const results: Uint8Array[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'paragraph') {
      const segments = collectInlineSegments(childEntry, ctx);

      let numberingId = 0;
      let headType = 0;
      if (listType === 'ordered') {
        numberingId = ctx.tables.numberings.intern({ format: 'decimal' }, 'decimal');
        headType = 2; // 번호
      } else {
        const bulletChar = getBulletChar(level);
        numberingId = ctx.tables.bullets.intern({ char: bulletChar }, `bullet-${bulletChar}`);
        headType = 3; // 글머리표
      }

      const paraShapeId = resolveParaShape(ctx, {
        align: childEntry.align as string | undefined,
        lineHeight: childEntry.line_height as number | undefined,
        indent: pxToHwpunit(20 * (level + 1)),
        headType,
        headLevel: Math.min(level, 6),
        numberingId,
      });

      if (isFirst) {
        const sectionRecords = buildSectionDef(ctx);
        results.push(...makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 0, sectionRecords));
        isFirst = false;
      } else {
        results.push(...makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 0));
      }
    } else {
      results.push(...convertNode(childId, ctx, isFirst));
      isFirst = false;
    }
  }

  return results;
}

function getBulletChar(level: number): number {
  const bullets = [0x25_cf, 0x25_cb, 0x25_a0, 0x25_c6, 0x25_b6, 0x20_22]; // ●, ○, ■, ◆, ▶, •
  return bullets[level % bullets.length];
}

// --- blockquote → 표 시뮬레이션 ---

function convertBlockquoteNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const variant = (entry as { variant?: string }).variant ?? 'left_line';

  const paragraphs = collectParagraphsFromChildren(entry, ctx);

  if (variant === 'message_sent' || variant === 'message_received') {
    const isSent = variant === 'message_sent';
    const hex =
      resolveColorToHex(isSent ? 'ui.blockquote.message-sent' : 'ui.blockquote.message-received') ?? (isSent ? '248BF5' : 'E5E5EA');
    const fillColor = hexToColorref(hex);

    // message_sent: 흰색 텍스트
    if (isSent) {
      const whiteColor = hexToColorref('FFFFFF');
      for (const p of paragraphs) {
        for (const seg of p.segments) {
          const whiteCharEntry: CharShapeEntry = {
            fontId: ctx.defaultFontId,
            baseSize: ctx.defaultFontSizePt100,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            textColor: whiteColor,
            underlineColor: whiteColor,
            shadeColor: 0xff_ff_ff_ff,
            shadowColor: 0x00_b2_b2_b2,
            strikethroughColor: whiteColor,
            letterSpacing: 0,
          };
          seg.charShapeId = ctx.tables.charShapes.intern(whiteCharEntry, `white-text`);
        }
      }
    }

    const cellMarginsH = 1600 + 1600; // left + right
    const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
    const maxWidth = pxToHwpunit(contentWidthPx) * 0.75;
    const textWidth = estimateTextWidthHwp(paragraphs, ctx.defaultFontSizePt100) + cellMarginsH;
    const tableWidthHwp = Math.max(pxToHwpunit(contentWidthPx) * 0.2, Math.min(textWidth, maxWidth));
    const ratio = tableWidthHwp / pxToHwpunit(contentWidthPx);

    return makeSimpleTableFromParagraphs(
      paragraphs,
      ctx,
      isFirst,
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
        fillType: 1,
        fillColor,
      },
      { left: 1600, right: 1600, top: 800, bottom: 800 },
      { tableWidthRatio: ratio, tableAlign: isSent ? 'right' : 'left' },
    );
  }

  // left_line / left_quote variant
  const borderColor = variant === 'left_quote' ? hexToColorref('000000') : hexToColorref('CCCCCC');
  return makeSimpleTableFromParagraphs(
    paragraphs,
    ctx,
    isFirst,
    {
      leftType: 1, // 실선 (0=없음, 1=실선)
      rightType: 0,
      topType: 0,
      bottomType: 0,
      leftWidth: 10, // 1.0mm (DOCX 2.25pt ≈ 0.8mm)
      rightWidth: 0,
      topWidth: 0,
      bottomWidth: 0,
      leftColor: borderColor,
      rightColor: 0,
      topColor: 0,
      bottomColor: 0,
      fillType: 0,
      fillColor: 0,
    },
    { left: 2000, right: 400, top: 400, bottom: 400 },
  );
}

// --- callout → 표 시뮬레이션 ---

function convertCalloutNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const variant = (entry as { variant?: string }).variant ?? 'info';
  const colorKey = `ui.callout.${variant}`;
  const hex = resolveColorToHex(colorKey);
  const borderColor = hex ? hexToColorref(hex) : hexToColorref('CCCCCC');

  const bgColors: Record<string, string> = {
    info: 'DBEAFE',
    success: 'DCFCE7',
    warning: 'FFF7ED',
    danger: 'FEF2F2',
  };
  const bgFill = hexToColorref(bgColors[variant] ?? 'F3F4F6');

  const paragraphs = collectParagraphsFromChildren(entry, ctx);

  return makeSimpleTableFromParagraphs(
    paragraphs,
    ctx,
    isFirst,
    {
      leftType: 1,
      rightType: 1,
      topType: 1,
      bottomType: 1,
      leftWidth: 10, // ~1.0mm 두꺼운 왼쪽 테두리
      rightWidth: 1,
      topWidth: 1,
      bottomWidth: 1,
      leftColor: borderColor,
      rightColor: borderColor,
      topColor: borderColor,
      bottomColor: borderColor,
      fillType: 1,
      fillColor: bgFill,
    },
    { left: 1200, right: 1200, top: 800, bottom: 800 },
  );
}

// --- fold → 2행 표 ---

function convertFoldNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const titleSegments: InlineSegment[] = [];
  const contentParagraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'fold_title') {
      titleSegments.push({ text: '\u25B6 ', charShapeId: ctx.defaultCharShapeId }, ...collectInlineSegments(childEntry, ctx));
    } else if (childEntry.type === 'fold_content') {
      for (const contentChildId of childEntry.children ?? []) {
        const contentChild = ctx.nodes[contentChildId];
        if (!contentChild || contentChild.type !== 'paragraph') continue;
        contentParagraphs.push({
          segments: collectInlineSegments(contentChild, ctx),
          align: contentChild.align as string | undefined,
          lineHeight: contentChild.line_height as number | undefined,
        });
      }
    }
  }

  const subtleBorderColor = hexToColorref('E3E4EB');
  const bfEntry: BorderFillEntry = {
    leftType: 1,
    rightType: 1,
    topType: 1,
    bottomType: 1,
    leftWidth: 1,
    rightWidth: 1,
    topWidth: 1,
    bottomWidth: 1,
    leftColor: subtleBorderColor,
    rightColor: subtleBorderColor,
    topColor: subtleBorderColor,
    bottomColor: subtleBorderColor,
    fillType: 1,
    fillColor: hexToColorref('F3F4F9'),
  };

  const bfKey = `fold-title`;
  const borderFillId = ctx.tables.borderFills.intern(bfEntry, bfKey);

  // 본문 셀: 상단 보더 없음, 배경색 없음
  const bfContent: BorderFillEntry = {
    ...bfEntry,
    topType: 0,
    topWidth: 0,
    topColor: 0,
    fillType: 0,
    fillColor: 0,
  };
  const borderFillContentId = ctx.tables.borderFills.intern(bfContent, 'fold-content');

  return makeTwoRowTable(titleSegments, contentParagraphs, borderFillId, borderFillContentId, ctx, isFirst, {
    left: 1200,
    right: 1200,
    top: 800,
    bottom: 800,
  });
}

// --- embed → 하이퍼링크 텍스트 표 ---

function convertEmbedNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const embedId = entry.id as string | undefined;
  const embedData = embedId ? ctx.embeds.get(embedId) : undefined;

  if (!embedData) {
    return convertPlaceholderNode('[임베드]', ctx, isFirst);
  }

  const label = embedData.title || embedData.url;
  const linkUrl = /^https?:|^mailto:/i.test(embedData.url) ? embedData.url : undefined;

  // 하이퍼링크 스타일: 파란색 + 밑줄
  const linkBlue = hexToColorref('0563C1');
  const linkCharShape: CharShapeEntry = {
    fontId: ctx.defaultFontId,
    baseSize: ctx.defaultFontSizePt100,
    bold: false,
    italic: false,
    underline: true,
    strikethrough: false,
    textColor: linkBlue,
    underlineColor: linkBlue,
    shadeColor: 0xff_ff_ff_ff,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: 0x00_00_00_00,
    letterSpacing: 0,
  };
  const linkCharShapeId = ctx.tables.charShapes.intern(linkCharShape, `link-blue`);
  const segments: InlineSegment[] = [{ text: label, charShapeId: linkCharShapeId, link: linkUrl }];

  const subtleBorderColor = hexToColorref('E3E4EB');
  return makeSimpleTableFromParagraphs(
    [{ segments }],
    ctx,
    isFirst,
    {
      leftType: 1,
      rightType: 1,
      topType: 1,
      bottomType: 1,
      leftWidth: 1,
      rightWidth: 1,
      topWidth: 1,
      bottomWidth: 1,
      leftColor: subtleBorderColor,
      rightColor: subtleBorderColor,
      topColor: subtleBorderColor,
      bottomColor: subtleBorderColor,
      fillType: 0,
      fillColor: 0,
    },
    { left: 1600, right: 1600, top: 1000, bottom: 1000 },
    { tableWidthRatio: 0.5, tableAlign: 'center', contentAlign: 'center' },
  );
}

// --- placeholder ---

function convertPlaceholderNode(text: string, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  // 회색 텍스트로 된 간단한 표
  const grayTextColor = hexToColorref('999999');
  const grayCharEntry: CharShapeEntry = {
    fontId: ctx.defaultFontId,
    baseSize: ctx.defaultFontSizePt100,
    bold: false,
    italic: false,
    underline: false,
    strikethrough: false,
    textColor: grayTextColor,
    underlineColor: grayTextColor,
    shadeColor: 0xff_ff_ff_ff,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: grayTextColor,
    letterSpacing: 0,
  };
  const grayCharShapeId = ctx.tables.charShapes.intern(grayCharEntry, `gray-placeholder`);

  const segments: InlineSegment[] = [{ text, charShapeId: grayCharShapeId }];

  const subtleBorderColor = hexToColorref('E3E4EB');
  return makeSimpleTableFromParagraphs(
    [{ segments }],
    ctx,
    isFirst,
    {
      leftType: 1,
      rightType: 1,
      topType: 1,
      bottomType: 1,
      leftWidth: 1,
      rightWidth: 1,
      topWidth: 1,
      bottomWidth: 1,
      leftColor: subtleBorderColor,
      rightColor: subtleBorderColor,
      topColor: subtleBorderColor,
      bottomColor: subtleBorderColor,
      fillType: 1,
      fillColor: hexToColorref('F3F4F6'),
    },
    { left: 1600, right: 1600, top: 1000, bottom: 1000 },
    { tableWidthRatio: 0.5, tableAlign: 'center', contentAlign: 'center' },
  );
}

// --- table ---

function convertTableNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  // 표 → 인라인 개체로 삽입
  const proportion = (entry as { proportion?: number }).proportion ?? 1;
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(proportion, 1));

  // 행/열 수집
  const rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[] = [];
  for (const rowId of entry.children ?? []) {
    const rowEntry = ctx.nodes[rowId];
    if (!rowEntry || rowEntry.type !== 'table_row') continue;

    const cells: { paraRecords: Uint8Array[]; colWidth: number }[] = [];
    for (const cellId of rowEntry.children ?? []) {
      const cellEntry = ctx.nodes[cellId];
      if (!cellEntry || cellEntry.type !== 'table_cell') continue;

      const cellRecords = convertCellContent(cellEntry, ctx);
      const colWidth = (cellEntry.col_width as number | null) ?? 0;
      cells.push({ paraRecords: cellRecords, colWidth });
    }
    rows.push({ cells });
  }

  if (rows.length === 0) return [];

  const rowCount = rows.length;
  const colCount = Math.max(...rows.map((r) => r.cells.length));

  // 기본 테두리
  const borderColor = hexToColorref('CCCCCC');
  const bfEntry: BorderFillEntry = {
    leftType: 1,
    rightType: 1,
    topType: 1,
    bottomType: 1,
    leftWidth: 1,
    rightWidth: 1,
    topWidth: 1,
    bottomWidth: 1,
    leftColor: borderColor,
    rightColor: borderColor,
    topColor: borderColor,
    bottomColor: borderColor,
    fillType: 0,
    fillColor: 0,
  };
  const tableBorderFillId = ctx.tables.borderFills.intern(bfEntry, 'table-default');
  const cellBorderFillId = tableBorderFillId;

  return makeTableRecords({
    rows,
    rowCount,
    colCount,
    tableWidth,
    tableBorderFillId,
    cellBorderFillId,
    ctx,
    isFirst,
    cellMargins: { left: 800, right: 800, top: 400, bottom: 400 },
  });
}

function convertCellContent(cellEntry: NodeEntry, ctx: HwpConvertContext): Uint8Array[] {
  const records: Uint8Array[] = [];
  for (const childId of cellEntry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;
    if (childEntry.type === 'paragraph') {
      const segments = collectInlineSegments(childEntry, ctx);
      const paraShapeId = resolveParaShape(ctx, {
        align: childEntry.align as string | undefined,
        lineHeight: childEntry.line_height as number | undefined,
      });
      records.push(...makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 2));
    }
  }
  if (records.length === 0) {
    records.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }
  return records;
}

// --- image ---

function convertImageNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const nodeId = entry.id as string | undefined;
  if (!nodeId) return convertPlaceholderNode('[이미지]', ctx, isFirst);

  const asset = ctx.assets.get(nodeId);
  if (!asset || asset.width <= 0 || asset.height <= 0) {
    return convertPlaceholderNode('[이미지를 불러올 수 없습니다]', ctx, isFirst);
  }

  const proportion = (entry as { proportion?: number }).proportion ?? 1;
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const displayWidthPx = contentWidthPx * Math.min(proportion, 1);
  const displayHeightPx = displayWidthPx * (asset.height / asset.width);

  const origWidth = pxToHwpunit(asset.width);
  const origHeight = pxToHwpunit(asset.height);
  const displayWidth = pxToHwpunit(displayWidthPx);
  const displayHeight = pxToHwpunit(displayHeightPx);

  // BinData 등록
  const ext = mapFormat(asset.format);
  const binDataId = ctx.tables.binData.intern({ extension: ext }, nodeId);

  const instanceId = ++ctx.instanceCounter;
  const records: Uint8Array[] = [];

  // 이미지 삽입 문단 (중앙정렬, isFirst일 때 section 정의 포함)
  const centerParaShapeId = resolveParaShape(ctx, { align: 'center' });
  records.push(
    ...makeInlineObjectParagraph(ctx, 0, 'gso ', {
      sectionRecords: isFirst ? buildSectionDef(ctx) : undefined,
      paraShapeId: centerParaShapeId,
    }),
    ...makeImageRecords(origWidth, origHeight, displayWidth, displayHeight, binDataId, instanceId),
  );

  return records;
}

// --- 표 레코드 생성 유틸 ---

function makeSimpleTableFromParagraphs(
  paragraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[],
  ctx: HwpConvertContext,
  isFirst: boolean,
  bfEntry: BorderFillEntry,
  cellMargins?: CellMargins,
  opts?: { tableWidthRatio?: number; tableAlign?: string; contentAlign?: string },
): Uint8Array[] {
  const borderFillId = ctx.tables.borderFills.intern(bfEntry, JSON.stringify(bfEntry));
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const ratio = opts?.tableWidthRatio ?? 1;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(ratio, 1));

  // 셀 내부 문단 레코드 생성
  const cellRecords: Uint8Array[] = [];
  for (const p of paragraphs) {
    const paraShapeId = resolveParaShape(ctx, {
      align: opts?.contentAlign ?? p.align,
      lineHeight: p.lineHeight,
    });
    cellRecords.push(...makeParagraph(p.segments, paraShapeId, ctx.defaultCharShapeId, 2));
  }
  if (cellRecords.length === 0) {
    cellRecords.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }

  const rows = [{ cells: [{ paraRecords: cellRecords, colWidth: tableWidth }] }];
  return makeTableRecords({
    rows,
    rowCount: 1,
    colCount: 1,
    tableWidth,
    tableBorderFillId: borderFillId,
    cellBorderFillId: borderFillId,
    ctx,
    isFirst,
    cellMargins,
    tableAlign: opts?.tableAlign,
  });
}

function makeTwoRowTable(
  titleSegments: InlineSegment[],
  contentParagraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[],
  titleBorderFillId: number,
  contentBorderFillId: number,
  ctx: HwpConvertContext,
  isFirst: boolean,
  cellMargins?: CellMargins,
): Uint8Array[] {
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx);

  const titleParaShapeId = resolveParaShape(ctx, {});
  const titleRecords = makeParagraph(titleSegments, titleParaShapeId, ctx.defaultCharShapeId, 2);

  const contentRecords: Uint8Array[] = [];
  for (const p of contentParagraphs) {
    const psId = resolveParaShape(ctx, { align: p.align, lineHeight: p.lineHeight });
    contentRecords.push(...makeParagraph(p.segments, psId, ctx.defaultCharShapeId, 2));
  }
  if (contentRecords.length === 0) {
    contentRecords.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }

  const rows = [
    { cells: [{ paraRecords: titleRecords, colWidth: tableWidth }] },
    { cells: [{ paraRecords: contentRecords, colWidth: tableWidth }] },
  ];

  // TABLE 레코드에는 fill 없는 border_fill 사용, 셀별로 개별 border_fill 적용
  const emptyBfId = ctx.tables.borderFills.intern(
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
    'table-no-fill',
  );

  return makeTableRecords({
    rows,
    rowCount: 2,
    colCount: 1,
    tableWidth,
    tableBorderFillId: emptyBfId,
    cellBorderFillId: emptyBfId,
    ctx,
    isFirst,
    cellMargins,
    perRowBorderFillIds: [titleBorderFillId, contentBorderFillId],
  });
}

type TableRecordsOpts = {
  rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[];
  rowCount: number;
  colCount: number;
  tableWidth: number;
  tableBorderFillId: number;
  cellBorderFillId: number;
  ctx: HwpConvertContext;
  isFirst: boolean;
  cellMargins?: CellMargins;
  tableAlign?: string;
  perRowBorderFillIds?: number[];
};

function makeTableRecords(opts: TableRecordsOpts): Uint8Array[] {
  const {
    rows,
    rowCount,
    colCount,
    tableWidth,
    tableBorderFillId,
    cellBorderFillId,
    ctx,
    isFirst,
    cellMargins,
    tableAlign,
    perRowBorderFillIds,
  } = opts;
  const instanceId = ++ctx.instanceCounter;

  // 표 삽입 문단 (첫 번째면 섹션 정의도 같은 문단에 합침)
  const sectionRecords = isFirst ? buildSectionDef(ctx) : undefined;
  const paraShapeId = tableAlign ? resolveParaShape(ctx, { align: tableAlign }) : undefined;
  return [
    ...makeInlineObjectParagraph(ctx, 0, 'tbl ', { sectionRecords, paraShapeId }),
    ...buildTableCoreRecords(
      rows,
      rowCount,
      colCount,
      tableWidth,
      tableBorderFillId,
      cellBorderFillId,
      instanceId,
      cellMargins,
      perRowBorderFillIds,
    ),
  ];
}

// --- 전체 BodyText 스트림 ---

export function buildBodyStream(ctx: HwpConvertContext): Uint8Array {
  const rootId = Object.keys(ctx.nodes).find((id) => ctx.nodes[id].type === 'root');
  if (!rootId) throw new Error('Root node not found');

  const rootEntry = ctx.nodes[rootId];
  const children = rootEntry.children ?? [];
  const records: Uint8Array[] = [];

  if (children.length === 0) {
    // 빈 문서: 섹션 정의만 포함한 빈 문단
    const sectionRecords = buildSectionDef(ctx);
    records.push(...makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, sectionRecords));
  } else {
    let isFirst = true;
    for (const childId of children) {
      records.push(...convertNode(childId, ctx, isFirst));
      isFirst = false;
    }
  }

  // 마지막 문단의 PARA_HEADER nchars에 bit 31 설정 (한/글 요구사항)
  setLastParagraphFlag(records);

  return concat(...records);
}

/** 마지막 PARA_HEADER의 nchars bit 31을 설정 */
function setLastParagraphFlag(records: Uint8Array[]): void {
  for (let i = records.length - 1; i >= 0; i--) {
    const rec = records[i];
    if (rec.byteLength < 8) continue; // 최소 4(헤더) + 4(nchars)
    const view = new DataView(rec.buffer, rec.byteOffset, rec.byteLength);
    const header = view.getUint32(0, true);
    const tagId = header & 0x3_ff;
    if (tagId === HWPTAG.PARA_HEADER) {
      const nchars = view.getUint32(4, true);
      view.setUint32(4, nchars | 0x80_00_00_00, true);
      return;
    }
  }
}
