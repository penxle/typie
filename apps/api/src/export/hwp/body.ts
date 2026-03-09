// cspell:ignore Hwpunit Segs
import { traverse } from '../core/traverse';
import {
  convertBlockquoteNode,
  convertCalloutNode,
  convertEmbedNode,
  convertFoldNode,
  convertPlaceholderNode,
  makeHorizontalRule,
} from './blocks';
import { convertImageNode } from './image';
import { buildSectionDef, collectInlineSegments, makeParagraph, setLastParagraphFlag } from './paragraph';
import { concat, pxToHwpunit } from './records';
import { resolveParaShape } from './styles';
import { convertTableNode } from './table';
import type { ParsedDocument } from '../core/document';
import type { NodeVisitor } from '../core/traverse';
import type { NodeEntry } from '../core/types';
import type { HwpConvertContext } from './types';

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

const hwpVisitor: NodeVisitor<HwpConvertContext, Uint8Array[]> = {
  paragraph: (node, ctx) => {
    const hwpSegs = collectInlineSegments(node.attrs as NodeEntry, ctx);
    const sectionRecords = consumeSectionDef(ctx);
    const listCtx = ctx.listStack.at(-1);

    if (listCtx) {
      const listType = listCtx.type;
      const level = listCtx.depth;

      let numberingId = 0;
      let headType = 0;
      if (listType === 'ordered') {
        numberingId = ctx.tables.numberings.intern({ format: 'decimal' }, 'decimal');
        headType = 2;
      } else {
        const bulletChar = getBulletChar(level);
        numberingId = ctx.tables.bullets.intern({ char: bulletChar }, `bullet-${bulletChar}`);
        headType = 3;
      }

      const paraShapeId = resolveParaShape(ctx, {
        align: node.attrs.align as string | undefined,
        lineHeight: node.attrs.line_height as number | undefined,
        indent: pxToHwpunit(20 * (level + 1)),
        headType,
        headLevel: Math.min(level, 6),
        numberingId,
      });

      return makeParagraph(hwpSegs, paraShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
    }

    const paraShapeId = resolveParaShape(ctx, {
      align: node.attrs.align as string | undefined,
      lineHeight: node.attrs.line_height as number | undefined,
      indent: ctx.paragraphIndentHwp,
    });

    return makeParagraph(hwpSegs, paraShapeId, ctx.defaultCharShapeId, 0, sectionRecords);
  },

  table: (entry, _convertChildren, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertTableNode(entry, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  image: (node, _asset, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertImageNode(node.attrs as NodeEntry, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  file: (_node, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertPlaceholderNode('[파일]', ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  embed: (id, data, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    // Reconstruct a minimal NodeEntry for the existing handler
    const entry = { type: 'embed', id } as NodeEntry;
    const result = convertEmbedNode(entry, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  archived: (_node, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertPlaceholderNode('[보관된 블록]', ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  horizontalRule: (ctx) => {
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

  blockquote: (entry, _variant, _convertChildren, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertBlockquoteNode(entry, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  callout: (entry, _variant, _convertChildren, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertCalloutNode(entry, ctx, isFirst);
    if (isFirst) ctx.sectionDefEmitted = true;
    return result;
  },

  fold: (entry, _convertChildren, ctx) => {
    const isFirst = !ctx.sectionDefEmitted;
    const result = convertFoldNode(entry, ctx, isFirst);
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

export function buildBodyStream(doc: ParsedDocument, ctx: HwpConvertContext): Uint8Array {
  const bodyChunks = traverse(doc, hwpVisitor, ctx);
  const allRecords = bodyChunks.flat();

  if (allRecords.length === 0) {
    allRecords.push(...makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx)));
  }

  setLastParagraphFlag(allRecords);
  return concat(...allRecords);
}
