// spell-checker:words HWPTAG HWPUNIT secd cold tdut tudt
import { allocate, ctrlId, HWPTAG, makeRecord, pxToHwpunit } from './records';
import { resolveCharShape, resolveRubyCharShape } from './styles';
import type { Annotation, HwpConvertContext, InlineSegment, NodeEntry, TextSegment } from './types';

/** PARA_LINE_SEG (36바이트) 기본값 — 한/글이 재계산하지만 초기 힌트 필요 */
function makeDefaultParaLineSeg(level: number): Uint8Array {
  const { buf, view } = allocate(36);
  view.setInt32(8, 1000, true); // 줄의 높이
  view.setInt32(12, 1000, true); // 텍스트 부분의 높이
  view.setInt32(16, 850, true); // 베이스라인까지 거리
  view.setInt32(20, 600, true); // 줄간격
  view.setUint32(32, 0x00_06_00_00, true); // 태그: 첫+마지막 세그먼트
  return makeRecord(HWPTAG.PARA_LINE_SEG, level, buf);
}

export function collectParagraphsFromChildren(
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

export function collectInlineSegments(entry: NodeEntry, ctx: HwpConvertContext): InlineSegment[] {
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

export function makeParagraph(
  segments: InlineSegment[],
  paraShapeId: number,
  defaultCharShapeId: number,
  level: number,
  extraCtrlRecords?: Uint8Array[],
): Uint8Array[] {
  const records: Uint8Array[] = [];
  const hasSectionDef = extraCtrlRecords && extraCtrlRecords.length > 0;

  const textParts: number[] = [];
  const charShapePairs: { pos: number; id: number }[] = [];
  let currentPos = 0;

  // 구역/단 정의: secd(8 WCHAR) + cold(8 WCHAR)
  if (hasSectionDef) {
    charShapePairs.push({ pos: currentPos, id: defaultCharShapeId });
    const secdId = ctrlId('secd');
    textParts.push(2, secdId & 0xff_ff, (secdId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    const coldId = ctrlId('cold');
    textParts.push(2, coldId & 0xff_ff, (coldId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    currentPos += 16;
  }

  const linkCtrlRecords: Uint8Array[] = [];
  const rubyCtrlRecords: Uint8Array[] = [];
  let hasFieldControl = false;
  let hasRubyControl = false;

  for (const seg of segments) {
    if (seg.text.length === 0) continue;

    if (seg.ruby) {
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
      if (charShapePairs.length === 0) {
        charShapePairs.push({ pos: currentPos, id: defaultCharShapeId });
      }
      const hlkId = ctrlId('%hlk');
      textParts.push(3, hlkId & 0xff_ff, (hlkId >> 16) & 0xff_ff, 0, 0, 0, 0, 3);
      currentPos += 8;
    }

    charShapePairs.push({ pos: currentPos, id: seg.charShapeId });

    for (const ch of seg.text) {
      if (ch === '\n') {
        textParts.push(10);
        currentPos += 1;
      } else {
        textParts.push(ch.codePointAt(0) ?? 0);
        currentPos += 1;
      }
    }

    if (seg.link) {
      textParts.push(4);
      currentPos += 1;
      const hlkId = ctrlId('%hlk');
      const hlkIdNoPercent = hlkId & 0x00_ff_ff_ff;
      textParts.push(hlkIdNoPercent & 0xff_ff, (hlkIdNoPercent >> 16) & 0xff_ff, 0, 0, 0, 0);
      currentPos += 6;
      textParts.push(4);
      currentPos += 1;
      linkCtrlRecords.push(buildHyperlinkCtrlHeader(seg.link, level + 1, currentPos));
    }
  }

  const isEmpty = currentPos === 0;
  textParts.push(13);
  currentPos += 1;

  if (charShapePairs.length === 0) {
    charShapePairs.push({ pos: 0, id: defaultCharShapeId });
  }

  const mergedPairs: { pos: number; id: number }[] = [];
  for (const pair of charShapePairs) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    if (mergedPairs.length > 0 && mergedPairs.at(-1)!.id === pair.id) continue;
    mergedPairs.push(pair);
  }

  // PARA_HEADER (24바이트)
  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, currentPos, true);
  let controlMask = 0;
  if (hasSectionDef) controlMask |= 0x04;
  if (hasFieldControl) controlMask |= 0x18;
  if (hasRubyControl) controlMask |= 0x80_00_00; // bit 23 = ruby "tdut"
  headerView.setUint32(4, controlMask, true);
  headerView.setUint16(8, paraShapeId, true);
  headerView.setUint8(10, 0);
  headerView.setUint8(11, hasSectionDef ? 0x03 : 0); // column_break_type: 구역+다단
  headerView.setUint16(12, mergedPairs.length, true);
  headerView.setUint16(14, 0, true);
  headerView.setUint16(16, 1, true);
  headerView.setUint32(18, 0, true);
  headerView.setUint16(22, 0, true);
  records.push(makeRecord(HWPTAG.PARA_HEADER, level, headerBuf));

  // PARA_TEXT — 빈 단락(nchars=1)에서는 생략
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

  if (extraCtrlRecords) {
    records.push(...extraCtrlRecords);
  }
  if (rubyCtrlRecords.length > 0) {
    records.push(...rubyCtrlRecords);
  }
  if (linkCtrlRecords.length > 0) {
    records.push(...linkCtrlRecords);
  }

  return records;
}

export function makeEmptyParagraph(paraShapeId: number, charShapeId: number, level: number): Uint8Array[] {
  return makeParagraph([], paraShapeId, charShapeId, level);
}

function buildHyperlinkCtrlHeader(url: string, level: number, fieldId: number): Uint8Array {
  const escapedUrl = url.replaceAll(':', String.raw`\:`);
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
  buf[offset] = 0;
  offset += 1;
  view.setUint16(offset, cmdLen, true);
  offset += 2;
  for (let i = 0; i < cmdLen; i++) {
    view.setUint16(offset, command.codePointAt(i) ?? 0, true);
    offset += 2;
  }
  view.setUint32(offset, fieldId, true);

  return makeRecord(HWPTAG.CTRL_HEADER, level, buf);
}

/** Ruby(덧말, "tdut") CTRL_HEADER */
function buildRubyCtrlHeader(baseText: string, rubyText: string, rubyCharShapeId: number, level: number): Uint8Array {
  const str0Bytes = baseText.length * 2;
  const str1Bytes = rubyText.length * 2;
  // ctrlId(4) + str[0..9](count-prefixed UTF-16) + trailing charShapeId(4)
  const totalSize = 4 + (2 + str0Bytes) + (2 + str1Bytes) + 8 * 2 + 4;
  const { buf, view } = allocate(totalSize);

  let offset = 0;
  view.setUint32(offset, ctrlId('tdut'), true);
  offset += 4;

  view.setUint16(offset, baseText.length, true);
  offset += 2;
  for (let i = 0; i < baseText.length; i++) {
    view.setUint16(offset, baseText.codePointAt(i) ?? 0, true);
    offset += 2;
  }

  view.setUint16(offset, rubyText.length, true);
  offset += 2;
  for (let i = 0; i < rubyText.length; i++) {
    view.setUint16(offset, rubyText.codePointAt(i) ?? 0, true);
    offset += 2;
  }

  // str[2]-str[9]: empty
  for (let i = 0; i < 8; i++) {
    view.setUint16(offset, 0, true);
    offset += 2;
  }

  view.setUint32(offset, rubyCharShapeId, true);

  return makeRecord(HWPTAG.CTRL_HEADER, level, buf);
}

export function makePageBreakParagraph(paraShapeId: number, charShapeId: number, level: number): Uint8Array[] {
  const records: Uint8Array[] = [];

  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, 1, true);
  headerView.setUint16(8, paraShapeId, true);
  headerView.setUint8(11, 0x04); // page break
  headerView.setUint16(12, 1, true);
  headerView.setUint16(16, 1, true);
  records.push(makeRecord(HWPTAG.PARA_HEADER, level, headerBuf));

  const { buf: csBuf, view: csView } = allocate(8);
  csView.setUint32(0, 0, true);
  csView.setUint32(4, charShapeId, true);
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, level + 1, csBuf), makeDefaultParaLineSeg(level + 1));

  return records;
}

/** char 11 (그리기 개체) 1개를 포함하는 인라인 개체 문단 */
export function makeInlineObjectParagraph(
  ctx: HwpConvertContext,
  level: number,
  ctrlIdStr: string,
  opts?: { sectionRecords?: Uint8Array[]; paraShapeId?: number },
): Uint8Array[] {
  const records: Uint8Array[] = [];
  const sectionRecords = opts?.sectionRecords;
  const hasSectionDef = sectionRecords && sectionRecords.length > 0;

  const textParts: number[] = [];

  if (hasSectionDef) {
    const secdId = ctrlId('secd');
    textParts.push(2, secdId & 0xff_ff, (secdId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
    const coldId = ctrlId('cold');
    textParts.push(2, coldId & 0xff_ff, (coldId >> 16) & 0xff_ff, 0, 0, 0, 0, 2);
  }

  const id = ctrlId(ctrlIdStr);
  textParts.push(11, id & 0xff_ff, (id >> 16) & 0xff_ff, 0, 0, 0, 0, 11, 13);

  const nchars = textParts.length;

  const { buf: headerBuf, view: headerView } = allocate(24);
  headerView.setUint32(0, nchars, true);
  headerView.setUint32(4, 0x08_00, true); // control_mask: bit 11 = 그리기 개체/표
  headerView.setUint16(8, opts?.paraShapeId ?? ctx.defaultParaShapeId, true);
  headerView.setUint16(12, 1, true);
  headerView.setUint16(16, 1, true);
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

  if (hasSectionDef) {
    records.push(...sectionRecords);
  }

  return records;
}

export function buildSectionDef(ctx: HwpConvertContext): Uint8Array[] {
  const records: Uint8Array[] = [];
  const pl = ctx.pageLayout;

  // CTRL_HEADER ("secd") + 구역 정의 속성 (26바이트)
  const { buf: ctrlBuf, view: ctrlView } = allocate(4 + 26);
  ctrlView.setUint32(0, ctrlId('secd'), true);
  ctrlView.setUint32(14, 8000, true); // 기본 탭 간격 (8000 HWPUNIT)
  records.push(makeRecord(HWPTAG.CTRL_HEADER, 1, ctrlBuf));

  // PAGE_DEF (40바이트)
  const { buf: pageBuf, view: pageView } = allocate(40);
  pageView.setUint32(0, pxToHwpunit(pl.pageWidth), true);
  pageView.setUint32(4, pxToHwpunit(pl.pageHeight), true);
  pageView.setUint32(8, pxToHwpunit(pl.pageMarginLeft), true);
  pageView.setUint32(12, pxToHwpunit(pl.pageMarginRight), true);
  pageView.setUint32(16, pxToHwpunit(pl.pageMarginTop), true);
  pageView.setUint32(20, pxToHwpunit(pl.pageMarginBottom), true);
  records.push(makeRecord(HWPTAG.PAGE_DEF, 2, pageBuf));

  // FOOTNOTE_SHAPE — 각주 (28바이트)
  const { buf: fn1, view: fn1v } = allocate(28);
  fn1v.setUint16(8, 0x00_29, true); // ')' 장식 문자
  fn1v.setUint16(10, 1, true);
  fn1v.setUint32(12, 0xff_ff_ff_ff, true); // 구분선 길이 = 전체
  fn1v.setUint16(16, 850, true);
  fn1v.setUint16(18, 567, true);
  fn1v.setUint16(20, 283, true);
  fn1v.setUint8(22, 1); // 실선
  fn1v.setUint8(23, 1);
  records.push(makeRecord(HWPTAG.FOOTNOTE_SHAPE, 2, fn1));

  // FOOTNOTE_SHAPE — 미주 (28바이트)
  const { buf: fn2, view: fn2v } = allocate(28);
  fn2v.setUint16(8, 0x00_29, true);
  fn2v.setUint16(10, 1, true);
  fn2v.setUint32(12, 0x00_e0_2f_f8, true);
  fn2v.setUint16(16, 850, true);
  fn2v.setUint16(18, 567, true);
  fn2v.setUint8(22, 1);
  fn2v.setUint8(23, 1);
  records.push(makeRecord(HWPTAG.FOOTNOTE_SHAPE, 2, fn2));

  // PAGE_BORDER_FILL (14바이트) × 3 (양쪽/홀수/짝수)
  for (let i = 0; i < 3; i++) {
    const { buf: borderFill, view: borderFillView } = allocate(14);
    borderFillView.setUint16(0, 1, true);
    borderFillView.setUint16(4, 1417, true);
    borderFillView.setUint16(6, 1417, true);
    borderFillView.setUint16(8, 1417, true);
    borderFillView.setUint16(10, 1417, true);
    borderFillView.setUint16(12, 1, true);
    records.push(makeRecord(HWPTAG.PAGE_BORDER_FILL, 2, borderFill));
  }

  // CTRL_HEADER ("cold") — 단 정의 (12바이트)
  const { buf: coldBuf, view: coldView } = allocate(4 + 12);
  coldView.setUint32(0, ctrlId('cold'), true);
  coldView.setUint16(4, 0x10_04, true); // 단종류=일반, 단개수=1, 단너비동일=1
  records.push(makeRecord(HWPTAG.CTRL_HEADER, 1, coldBuf));

  return records;
}

/** 마지막 PARA_HEADER의 nchars bit 31 설정 */
export function setLastParagraphFlag(records: Uint8Array[]): void {
  for (let i = records.length - 1; i >= 0; i--) {
    const rec = records[i];
    if (rec.byteLength < 8) continue;
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
