// spell-checker:words HWPTAG HWPUNIT DBEAFE DCFCE
import { resolveColorToHex } from '../core/theme.ts';
import { buildSectionDef, collectInlineSegments, collectParagraphsFromChildren, makeParagraph } from './paragraph.ts';
import { allocate, ctrlId, hexToColorref, HWPTAG, makeRecord, pxToHwpunit } from './records.ts';
import { estimateTextWidthHwp, resolveParaShape } from './styles.ts';
import { makeSimpleTableFromParagraphs, makeTwoRowTable } from './table.ts';
import type { NodeEntry } from '../core/types.ts';
import type { BorderFillEntry, CharShapeEntry } from './doc-info.ts';
import type { HwpConvertContext, InlineSegment } from './types.ts';

export type ConvertNodeFn = (nodeId: string, ctx: HwpConvertContext, isFirst: boolean) => Uint8Array[];

// --- 목록 ---

export function convertListItem(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean, convertNode: ConvertNodeFn): Uint8Array[] {
  const currentList = ctx.listStack.at(-1);
  const listType = currentList?.type ?? 'bullet';
  const level = currentList?.depth ?? 0;
  const results: Uint8Array[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'paragraph') {
      const segments = collectInlineSegments(childEntry, ctx);

      let numberingId: number;
      let headType: number;
      if (listType === 'ordered') {
        numberingId = ctx.tables.numberings.intern({ format: 'decimal' }, 'decimal');
        headType = 2;
      } else {
        const bulletChar = getBulletChar(level);
        numberingId = ctx.tables.bullets.intern({ char: bulletChar }, `bullet-${bulletChar}`);
        headType = 3;
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
  const bullets = [0x25_cf, 0x25_cb, 0x25_a0, 0x25_c6, 0x25_b6, 0x20_22]; // ●○■◆▶‣
  return bullets[level % bullets.length];
}

// --- blockquote ---

export function convertBlockquoteNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const variant = (entry as { variant?: string }).variant ?? 'left_line';
  const paragraphs = collectParagraphsFromChildren(entry, ctx);

  if (variant === 'message_sent' || variant === 'message_received') {
    const isSent = variant === 'message_sent';
    const hex =
      resolveColorToHex(isSent ? 'ui.blockquote.message-sent' : 'ui.blockquote.message-received') ?? (isSent ? '248BF5' : 'E5E5EA');
    const fillColor = hexToColorref(hex);

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

    const cellMarginsH = 1600 + 1600;
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

  const borderColor = hexToColorref(variant === 'left_quote' ? '000000' : 'CCCCCC');
  return makeSimpleTableFromParagraphs(
    paragraphs,
    ctx,
    isFirst,
    {
      leftType: 1,
      rightType: 0,
      topType: 0,
      bottomType: 0,
      leftWidth: 10,
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

// --- callout ---

export function convertCalloutNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const variant = (entry as { variant?: string }).variant ?? 'info';
  const colorKey = `ui.callout.${variant}`;
  const hex = resolveColorToHex(colorKey);
  const borderColor = hexToColorref(hex || 'CCCCCC');

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
      leftWidth: 10,
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

// --- fold ---

export function convertFoldNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const titleSegments: InlineSegment[] = [];
  const contentParagraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'fold_title') {
      titleSegments.push({ text: '\u{25B6} ', charShapeId: ctx.defaultCharShapeId }, ...collectInlineSegments(childEntry, ctx));
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
  const borderFillId = ctx.tables.borderFills.intern(bfEntry, 'fold-title');

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

// --- embed ---

export function convertEmbedNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const embedId = entry.id as string | undefined;
  const embedData = embedId ? ctx.embeds.get(embedId) : undefined;

  if (!embedData) {
    return convertPlaceholderNode('[임베드]', ctx, isFirst);
  }

  const label = embedData.title || embedData.url;
  const linkUrl = /^https?:|^mailto:/i.test(embedData.url) ? embedData.url : undefined;

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

export function convertPlaceholderNode(text: string, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
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
  const grayCharShapeId = ctx.tables.charShapes.intern(grayCharEntry, 'gray-placeholder');

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

// --- 수평선 ---

/** GSO 컨트롤 헤더 (46바이트, 글자처럼 취급) */
function buildGsoCtrlHeader(width: number, height: number, instanceId: number): Uint8Array {
  const { buf, view } = allocate(46);
  view.setUint32(0, ctrlId('gso '), true);
  const attr = 0x01 | (2 << 3) | (2 << 8) | (3 << 21) | (4 << 15) | (2 << 18);
  view.setUint32(4, attr, true);
  view.setUint32(16, width, true);
  view.setUint32(20, height, true);
  view.setUint32(36, instanceId, true);
  view.setUint16(44, 0, true);
  return buf;
}

export function makeHorizontalRule(ctx: HwpConvertContext): Uint8Array[] {
  const contentWidth = pxToHwpunit(ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight);
  // 컨트롤 높이: fontSize × (2 - lineSpacing/100)
  const height = Math.max(pxToHwpunit(2), Math.round(ctx.defaultFontSizePt100 * (2 - ctx.defaultLineHeight / 100)));
  const instanceId = ++ctx.instanceCounter;

  const records: Uint8Array[] = [];

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

  const { buf: csBuf, view: csView } = allocate(8);
  csView.setUint32(4, ctx.defaultCharShapeId, true);
  records.push(makeRecord(HWPTAG.PARA_CHAR_SHAPE, 1, csBuf));

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

  // SHAPE_COMPONENT_LINE (20바이트)
  const { buf: lineBuf, view: lineView } = allocate(20);
  lineView.setInt32(8, contentWidth, true); // end_x
  records.push(makeRecord(HWPTAG.SHAPE_COMPONENT_LINE, 3, lineBuf));

  return records;
}

function buildLineShapeComponent(width: number, height: number): Uint8Array {
  const renderingInfo = buildRenderingInfo();
  // 8(ctrl_ids) + 42(개체요소속성) + 146(rendering) + 11(border) + 8(fill) + 24(textbox+shadow)
  const totalSize = 8 + 42 + renderingInfo.byteLength + 11 + 8 + 24;
  const { buf, view } = allocate(totalSize);
  view.setUint32(0, ctrlId('$lin'), true);
  view.setUint32(4, ctrlId('$lin'), true);
  let offset = 8;

  view.setUint16(offset + 10, 1, true); // local_version = 1
  offset += 12;
  view.setUint32(offset, width, true);
  offset += 4;
  view.setUint32(offset, height, true);
  offset += 4;
  view.setUint32(offset, width, true);
  offset += 4;
  view.setUint32(offset, height, true);
  offset += 4;
  offset += 4; // flags
  offset += 2; // rotation
  view.setInt32(offset, Math.floor(width / 2), true);
  offset += 4;
  view.setInt32(offset, Math.floor(height / 2), true);
  offset += 4;

  buf.set(renderingInfo, offset);
  offset += renderingInfo.byteLength;

  // 테두리 선 정보 (11바이트)
  view.setUint32(offset, hexToColorref('000000'), true);
  offset += 4;
  view.setInt16(offset, 100, true);
  offset += 2;
  view.setUint32(offset, 0x00_41_00_00, true); // 속성 (레퍼런스 값)
  // +4 outline_style = 0 (1바이트)

  // 채우기 (8바이트, type=0)
  // textbox + shadow (24바이트, 모두 0)

  return buf;
}

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
