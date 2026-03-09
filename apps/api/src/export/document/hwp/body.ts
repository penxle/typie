import {
  convertBlockquoteNode,
  convertCalloutNode,
  convertEmbedNode,
  convertFoldNode,
  convertListItem,
  convertPlaceholderNode,
  makeHorizontalRule,
} from './blocks';
import { convertImageNode } from './image';
import { buildSectionDef, collectInlineSegments, makePageBreakParagraph, makeParagraph, setLastParagraphFlag } from './paragraph';
import { concat } from './records';
import { resolveParaShape } from './styles';
import { convertTableNode } from './table';
import type { HwpConvertContext, NodeEntry } from './types';

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
        return makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx));
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
        const emptyPara = makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx));
        return [...emptyPara, ...makeHorizontalRule(ctx)];
      }
      return makeHorizontalRule(ctx);
    }

    case 'page_break': {
      if (isFirst) {
        return makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx));
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
      return convertListItem(entry, ctx, isFirst, convertNode);
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

export function buildBodyStream(ctx: HwpConvertContext): Uint8Array {
  const rootId = Object.keys(ctx.nodes).find((id) => ctx.nodes[id].type === 'root');
  if (!rootId) throw new Error('Root node not found');

  const rootEntry = ctx.nodes[rootId];
  const children = rootEntry.children ?? [];
  const records: Uint8Array[] = [];

  if (children.length === 0) {
    records.push(...makeParagraph([], ctx.defaultParaShapeId, ctx.defaultCharShapeId, 0, buildSectionDef(ctx)));
  } else {
    let isFirst = true;
    for (const childId of children) {
      records.push(...convertNode(childId, ctx, isFirst));
      isFirst = false;
    }
  }

  setLastParagraphFlag(records);
  return concat(...records);
}
