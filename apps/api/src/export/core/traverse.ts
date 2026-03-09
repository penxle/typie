import type { ParsedDocument } from './document';
import type {
  ArchivedData,
  EmbedInfo,
  FileData,
  ImageAsset,
  ImageData,
  InlineSegment,
  NodeEntry,
  ParagraphData,
  TextSegment,
} from './types';

export type ConvertChildrenFn<TOut> = (entry: NodeEntry) => TOut[];

export type NodeVisitor<TCtx, TOut> = {
  paragraph: (node: ParagraphData, ctx: TCtx) => TOut;
  table: (entry: NodeEntry, convertChildren: ConvertChildrenFn<TOut>, ctx: TCtx) => TOut;
  image: (node: ImageData, asset: ImageAsset, ctx: TCtx) => TOut;
  file: (node: FileData, ctx: TCtx) => TOut;
  embed: (id: string, data: EmbedInfo | undefined, ctx: TCtx) => TOut;
  archived: (node: ArchivedData, ctx: TCtx) => TOut;
  horizontalRule: (ctx: TCtx) => TOut;
  bulletList: (items: TOut[][], ctx: TCtx) => TOut;
  orderedList: (items: TOut[][], ctx: TCtx) => TOut;
  blockquote: (entry: NodeEntry, variant: string, convertChildren: ConvertChildrenFn<TOut>, ctx: TCtx) => TOut;
  callout: (entry: NodeEntry, variant: string, convertChildren: ConvertChildrenFn<TOut>, ctx: TCtx) => TOut;
  fold: (entry: NodeEntry, convertChildren: ConvertChildrenFn<TOut>, ctx: TCtx) => TOut;

  /** list children 변환 전에 호출 (HWP/DOCX: listStack push) */
  onEnterList?: (type: 'bullet' | 'ordered', depth: number, ctx: TCtx) => void;
  /** list children 변환 후에 호출 (HWP/DOCX: listStack pop) */
  onExitList?: (ctx: TCtx) => void;
};

export function traverse<TCtx, TOut>(doc: ParsedDocument, visitor: NodeVisitor<TCtx, TOut>, ctx: TCtx): TOut[] {
  const { nodes, images, embeds } = doc;

  const rootId = Object.keys(nodes).find((id) => nodes[id].type === 'root');
  if (!rootId) return [];
  const rootEntry = nodes[rootId];

  let listDepth = 0;

  return convertChildren(rootEntry);

  function convertChildren(entry: NodeEntry): TOut[] {
    const results: TOut[] = [];
    for (const childId of entry.children ?? []) {
      const result = convertNode(childId);
      if (result !== undefined) results.push(result);
    }
    return results;
  }

  function convertNode(nodeId: string): TOut | undefined {
    const entry = nodes[nodeId];
    if (!entry) return undefined;

    switch (entry.type) {
      case 'paragraph': {
        return visitor.paragraph(parseParagraph(entry, nodes), ctx);
      }

      case 'table': {
        return visitor.table(entry, convertChildren, ctx);
      }

      case 'image': {
        const asset = images.get(entry.id as string);
        if (!asset) return undefined;
        return visitor.image({ id: entry.id as string, attrs: entry }, asset, ctx);
      }

      case 'file': {
        return visitor.file({ id: entry.id as string, attrs: entry }, ctx);
      }

      case 'embed': {
        const id = entry.id as string;
        const data = embeds.get(id);
        return visitor.embed(id, data, ctx);
      }

      case 'archived': {
        return visitor.archived({ attrs: entry }, ctx);
      }

      case 'horizontal_rule': {
        return visitor.horizontalRule(ctx);
      }

      case 'bullet_list': {
        visitor.onEnterList?.('bullet', listDepth, ctx);
        listDepth++;
        const bulletItems = parseListItems(entry);
        listDepth--;
        visitor.onExitList?.(ctx);
        return visitor.bulletList(bulletItems, ctx);
      }

      case 'ordered_list': {
        visitor.onEnterList?.('ordered', listDepth, ctx);
        listDepth++;
        const orderedItems = parseListItems(entry);
        listDepth--;
        visitor.onExitList?.(ctx);
        return visitor.orderedList(orderedItems, ctx);
      }

      case 'blockquote': {
        const variant = (entry.variant as string) ?? 'left_line';
        return visitor.blockquote(entry, variant, convertChildren, ctx);
      }

      case 'callout': {
        const variant = (entry.variant as string) ?? 'info';
        return visitor.callout(entry, variant, convertChildren, ctx);
      }

      case 'fold': {
        return visitor.fold(entry, convertChildren, ctx);
      }

      default: {
        return undefined;
      }
    }
  }

  function parseListItems(listEntry: NodeEntry): TOut[][] {
    return (listEntry.children ?? []).map((itemId) => {
      const item = nodes[itemId];
      if (!item || item.type !== 'list_item') return [];
      return convertChildren(item);
    });
  }
}

function parseParagraph(entry: NodeEntry, nodes: Record<string, NodeEntry>): ParagraphData {
  const segments: InlineSegment[] = [];
  for (const childId of entry.children ?? []) {
    const child = nodes[childId];
    if (!child) continue;
    switch (child.type) {
      case 'text': {
        for (const seg of (child.text as { text: string; styles: unknown[]; annotations: unknown[] }[]) ?? []) {
          segments.push({
            type: 'text',
            text: seg.text,
            styles: (seg.styles ?? []) as TextSegment['styles'],
            annotations: (seg.annotations ?? []) as TextSegment['annotations'],
          });
        }
        break;
      }
      case 'hard_break': {
        segments.push({ type: 'hard_break' });
        break;
      }
      case 'page_break': {
        segments.push({ type: 'page_break' });
        break;
      }
    }
  }
  return { segments, attrs: entry };
}
