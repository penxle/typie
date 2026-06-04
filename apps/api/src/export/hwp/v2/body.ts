// cspell:ignore Hwpunit
import { traverseV2 } from '../../core/v2/traverse.ts';
import { makeHorizontalRule } from '../blocks.ts';
import { buildSectionDef, makePageBreakParagraph, makeParagraph, setLastParagraphFlag } from '../paragraph.ts';
import { concat, pxToHwpunit } from '../records.ts';
import { resolveParaShape } from '../styles.ts';
import { blockquoteToRecordsV2, calloutToRecordsV2, convertPlaceholderNodeV2, embedToRecordsV2, foldToRecordsV2 } from './blocks.ts';
import { imageToRecordsV2 } from './image.ts';
import { splitParagraphPartsV2 } from './paragraph.ts';
import { tableToRecordsV2 } from './table.ts';
import type { NodeVisitorV2, ParsedDocumentV2 } from '../../core/v2/types.ts';
import type { HwpConvertContext } from '../types.ts';

function consumeSectionDef(ctx: HwpConvertContext): Uint8Array[] | undefined {
  if (!ctx.sectionDefEmitted) {
    ctx.sectionDefEmitted = true;
    return buildSectionDef(ctx);
  }
  return undefined;
}

function getBulletChar(level: number): number {
  const bullets = [0x25_cf, 0x25_cb, 0x25_a0, 0x25_c6, 0x25_b6, 0x20_22]; // ●○■◆▶‣
  return bullets[level % bullets.length];
}

const hwpVisitorV2: NodeVisitorV2<HwpConvertContext, Uint8Array[]> = {
  paragraph: (p, ctx) => {
    const parts = splitParagraphPartsV2(p, ctx);
    const listCtx = ctx.listStack.at(-1);

    let paraShapeId: number;
    if (listCtx) {
      const level = listCtx.depth;
      let numberingId: number;
      let headType: number;
      if (listCtx.type === 'ordered') {
        numberingId = ctx.tables.numberings.intern({ format: 'decimal' }, 'decimal');
        headType = 2;
      } else {
        const bulletChar = getBulletChar(level);
        numberingId = ctx.tables.bullets.intern({ char: bulletChar }, `bullet-${bulletChar}`);
        headType = 3;
      }
      paraShapeId = resolveParaShape(ctx, {
        align: p.align,
        lineHeight: p.lineHeight,
        indent: pxToHwpunit(20 * (level + 1)),
        headType,
        headLevel: Math.min(level, 6),
        numberingId,
      });
    } else {
      paraShapeId = resolveParaShape(ctx, { align: p.align, lineHeight: p.lineHeight, indent: ctx.paragraphIndentHwp });
    }

    const records: Uint8Array[] = [];
    for (const [i, part] of parts.entries()) {
      const sectionRecords = i === 0 ? consumeSectionDef(ctx) : undefined;
      records.push(...makeParagraph(part.segments, paraShapeId, ctx.defaultCharShapeId, 0, sectionRecords));
      if (part.pageBreakAfter) {
        records.push(...makePageBreakParagraph(paraShapeId, ctx.defaultCharShapeId, 0));
      }
    }
    return records;
  },

  table: (t, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = tableToRecordsV2(t, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  image: (n, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = imageToRecordsV2(n, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  file: (_n, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertPlaceholderNodeV2('[파일]', ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  embed: (n, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = embedToRecordsV2(n.data, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  archived: (_n, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertPlaceholderNodeV2('[보관된 블록]', ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  horizontalRule: (_variant, ctx) => {
    const sectionRecords = consumeSectionDef(ctx);
    if (sectionRecords) {
      const emptyPara = makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
      return [...emptyPara, ...makeHorizontalRule(ctx)];
    }
    return makeHorizontalRule(ctx);
  },

  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  bulletList: (items) => items.flat(2),
  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  orderedList: (items) => items.flat(2),

  blockquote: (variant, children, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = blockquoteToRecordsV2(variant, children, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  callout: (variant, children, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = calloutToRecordsV2(variant, children, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  fold: (title, content, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = foldToRecordsV2(title, content, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  onEnterList: (type, depth, ctx) => {
    ctx.listStack.push({ type, depth });
  },

  onExitList: (ctx) => {
    ctx.listStack.pop();
  },
};

export function buildBodyStreamV2(parsed: ParsedDocumentV2, ctx: HwpConvertContext): Uint8Array {
  const bodyChunks = traverseV2(parsed, hwpVisitorV2, ctx);
  const allRecords = bodyChunks.flat();

  if (allRecords.length === 0) {
    allRecords.push(...makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx)));
  }

  setLastParagraphFlag(allRecords);
  return concat(...allRecords);
}
