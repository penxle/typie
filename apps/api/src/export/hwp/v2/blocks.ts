// spell-checker:words HWPTAG HWPUNIT DBEAFE DCFCE
import { resolveColorToHex } from '../../core/theme.ts';
import { buildSectionDef, makeEmptyParagraph, makeInlineObjectParagraph, makeParagraph } from '../paragraph.ts';
import { hexToColorref, pxToHwpunit } from '../records.ts';
import { resolveParaShape } from '../styles.ts';
import { makeTableRecords } from '../table.ts';
import { resolveCharShapeV2 } from './styles.ts';
import type { Run } from '../../core/v2/types.ts';
import type { BorderFillEntry, CharShapeEntry } from '../doc-info.ts';
import type { CellMargins } from '../table.ts';
import type { HwpConvertContext, InlineSegment } from '../types.ts';

type TableOpts = { tableWidthRatio?: number; tableAlign?: string };

function makeSingleCellTableV2(
  cellRecords: Uint8Array[],
  ctx: HwpConvertContext,
  isFirst: boolean,
  bfEntry: BorderFillEntry,
  cellMargins: CellMargins,
  opts?: TableOpts,
): Uint8Array[] {
  const borderFillId = ctx.tables.borderFills.intern(bfEntry, JSON.stringify(bfEntry));
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const ratio = opts?.tableWidthRatio ?? 1;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(ratio, 1));

  const records = cellRecords.length > 0 ? cellRecords : makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2);

  const rows = [{ cells: [{ paraRecords: records, colWidth: tableWidth }] }];
  const instanceId = ++ctx.instanceCounter;
  const sectionRecords = isFirst ? buildSectionDef(ctx) : undefined;
  const paraShapeId = opts?.tableAlign ? resolveParaShape(ctx, { align: opts.tableAlign }) : undefined;

  return [
    ...makeInlineObjectParagraph(ctx, 0, 'tbl ', { sectionRecords, paraShapeId }),
    ...makeTableRecords(rows, 1, 1, tableWidth, borderFillId, borderFillId, instanceId, cellMargins),
  ];
}

function makeLabelCellRecords(segments: InlineSegment[], ctx: HwpConvertContext, align?: string): Uint8Array[] {
  const paraShapeId = resolveParaShape(ctx, { align });
  return makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 2);
}

export function blockquoteToRecordsV2(variant: string, children: Uint8Array[][], ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const cellRecords = children.flat();

  if (variant === 'message_sent' || variant === 'message_received') {
    const isSent = variant === 'message_sent';
    const hex =
      resolveColorToHex(isSent ? 'ui.blockquote.message-sent' : 'ui.blockquote.message-received') ?? (isSent ? '248BF5' : 'E5E5EA');
    const fillColor = hexToColorref(hex);

    return makeSingleCellTableV2(
      cellRecords,
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
      { tableWidthRatio: 0.6, tableAlign: isSent ? 'right' : 'left' },
    );
  }

  const borderColor = variant === 'left_quote' ? hexToColorref('000000') : hexToColorref('CCCCCC');
  return makeSingleCellTableV2(
    cellRecords,
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

export function calloutToRecordsV2(variant: string, children: Uint8Array[][], ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const hex = resolveColorToHex(`ui.callout.${variant}`);
  const borderColor = hex ? hexToColorref(hex) : hexToColorref('CCCCCC');

  const bgColors: Record<string, string> = {
    info: 'DBEAFE',
    success: 'DCFCE7',
    warning: 'FFF7ED',
    danger: 'FEF2F2',
  };
  const bgFill = hexToColorref(bgColors[variant] ?? 'F3F4F6');

  return makeSingleCellTableV2(
    children.flat(),
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

export function foldToRecordsV2(title: Run[], content: Uint8Array[][], ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const titleSegments: InlineSegment[] = [{ text: '▶ ', charShapeId: ctx.defaultCharShapeId }];
  for (const run of title) {
    titleSegments.push({ text: run.text, charShapeId: resolveCharShapeV2(run.style, ctx), link: run.style.link });
  }

  const subtleBorderColor = hexToColorref('E3E4EB');
  const titleBf: BorderFillEntry = {
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
  const titleBfId = ctx.tables.borderFills.intern(titleBf, 'fold-title');
  const contentBf: BorderFillEntry = { ...titleBf, topType: 0, topWidth: 0, topColor: 0, fillType: 0, fillColor: 0 };
  const contentBfId = ctx.tables.borderFills.intern(contentBf, 'fold-content');

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

  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx);

  const titleParaShapeId = resolveParaShape(ctx, {});
  const titleRecords = makeParagraph(titleSegments, titleParaShapeId, ctx.defaultCharShapeId, 2);
  const contentRecords = content.flat();
  const contentCell = contentRecords.length > 0 ? contentRecords : makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2);

  const rows = [
    { cells: [{ paraRecords: titleRecords, colWidth: tableWidth }] },
    { cells: [{ paraRecords: contentCell, colWidth: tableWidth }] },
  ];

  const instanceId = ++ctx.instanceCounter;
  const sectionRecords = isFirst ? buildSectionDef(ctx) : undefined;

  return [
    ...makeInlineObjectParagraph(ctx, 0, 'tbl ', { sectionRecords }),
    ...makeTableRecords(rows, 2, 1, tableWidth, emptyBfId, emptyBfId, instanceId, { left: 1200, right: 1200, top: 800, bottom: 800 }, [
      titleBfId,
      contentBfId,
    ]),
  ];
}

export function embedToRecordsV2(
  data: { url: string; title: string | null } | undefined,
  ctx: HwpConvertContext,
  isFirst: boolean,
): Uint8Array[] {
  if (!data) {
    return convertPlaceholderNodeV2('[임베드]', ctx, isFirst);
  }

  const label = data.title || data.url;
  const linkUrl = /^https?:|^mailto:/i.test(data.url) ? data.url : undefined;

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
  return makeSingleCellTableV2(
    makeLabelCellRecords(segments, ctx, 'center'),
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
    { tableWidthRatio: 0.5, tableAlign: 'center' },
  );
}

export function convertPlaceholderNodeV2(text: string, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
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
  return makeSingleCellTableV2(
    makeLabelCellRecords(segments, ctx, 'center'),
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
    { tableWidthRatio: 0.5, tableAlign: 'center' },
  );
}
