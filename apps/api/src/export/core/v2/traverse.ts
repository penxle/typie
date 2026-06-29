import { resolveColorToHex } from '../theme.ts';
import { resolveRunStyle } from './style.ts';
import type { Alignment, PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/server';
import type { EmbedInfo, ImageAsset } from '../types.ts';
import type { Inline, NodeVisitorV2, ParagraphV2, Run, TableV2 } from './types.ts';

export function traverseV2<TCtx, TOut>(
  parsed: {
    plain: PlainDoc;
    root: PlainNodeEntry;
    defaults: { fontFamily: string; fontSizePt100: number };
    images: Map<string, ImageAsset>;
    embeds: Map<string, EmbedInfo>;
  },
  visitor: NodeVisitorV2<TCtx, TOut>,
  ctx: TCtx,
): TOut[] {
  const defaults = parsed.defaults;
  let listDepth = 0;
  return convertChildren(parsed.root);

  function convertChildren(e: PlainNodeEntry): TOut[] {
    const out: TOut[] = [];
    for (const child of e.children) {
      const r = convertNode(child);
      if (r !== undefined) out.push(r);
    }
    return out;
  }
  function parseParagraph(e: PlainNodeEntry): ParagraphV2 {
    const inlines: Inline[] = [];
    for (const c of e.children) {
      switch (c.node.type) {
        case 'text': {
          inlines.push({ type: 'run', run: { text: c.node.text, style: resolveRunStyle(c.modifiers, defaults) } });
          break;
        }
        case 'hard_break': {
          inlines.push({ type: 'hard_break' });
          break;
        }
        case 'page_break': {
          inlines.push({ type: 'page_break' });
          break;
        }
        case 'tab': {
          inlines.push({ type: 'tab' });
          break;
        }
      }
    }
    const alignMod = e.modifiers['alignment'];
    const lhMod = e.modifiers['line_height'];
    return {
      inlines,
      align: (alignMod?.type === 'alignment' ? alignMod.value : 'left') as Alignment,
      lineHeight: lhMod?.type === 'line_height' ? lhMod.value : 160,
    };
  }
  function buildTable(e: PlainNodeEntry): TableV2<TOut> {
    const rows = e.children.map((row) => {
      const cells = row.children.map((cell) => {
        const node = cell.node;
        const cellBg = cell.modifiers['background_color'];
        return {
          children: convertChildren(cell),
          colWidth: node.type === 'table_cell' ? (node.col_width ?? undefined) : undefined,
          backgroundColorHex:
            cellBg?.type === 'background_color' && cellBg.value !== 'none' ? resolveColorToHex(`bg.${cellBg.value}`) : undefined,
        };
      });
      return { cells };
    });
    const tnode = e.node;
    return {
      rows,
      borderStyle: tnode.type === 'table' ? (tnode.border_style ?? 'solid') : 'solid',
      proportion: tnode.type === 'table' ? (tnode.proportion ?? 100) / 100 : 1,
    };
  }
  function convertNode(e: PlainNodeEntry): TOut | undefined {
    const v = visitor;
    switch (e.node.type) {
      case 'paragraph': {
        return v.paragraph(parseParagraph(e), ctx);
      }
      case 'table': {
        return v.table(buildTable(e), ctx);
      }
      case 'image': {
        const imgId = e.node.id ?? undefined;
        const asset = imgId ? parsed.images.get(imgId) : undefined;
        if (!imgId || !asset) return undefined;
        return v.image({ id: imgId, proportion: (e.node.proportion ?? 100) / 100, asset }, ctx);
      }
      case 'file': {
        return e.node.id ? v.file({ id: e.node.id }, ctx) : undefined;
      }
      case 'embed': {
        const eid = e.node.id ?? undefined;
        if (!eid) return undefined;
        return v.embed({ id: eid, data: parsed.embeds.get(eid) }, ctx);
      }
      case 'archived': {
        return e.node.id ? v.archived({ id: e.node.id }, ctx) : undefined;
      }
      case 'horizontal_rule': {
        return v.horizontalRule(e.node.variant ?? 'line', ctx);
      }
      case 'bullet_list':
      case 'ordered_list': {
        const kind = e.node.type === 'bullet_list' ? 'bullet' : 'ordered';
        v.onEnterList?.(kind, listDepth, ctx);
        listDepth++;
        const items = e.children.map((item) => (item.node.type === 'list_item' ? convertChildren(item) : []));
        listDepth--;
        v.onExitList?.(ctx);
        return kind === 'bullet' ? v.bulletList(items, ctx) : v.orderedList(items, ctx);
      }
      case 'blockquote': {
        return v.blockquote(e.node.variant ?? 'left_line', convertChildren(e), ctx);
      }
      case 'callout': {
        return v.callout(e.node.variant ?? 'info', convertChildren(e), ctx);
      }
      case 'fold': {
        const title: Run[] = [];
        let content: TOut[] = [];
        for (const c of e.children) {
          if (c.node.type === 'fold_title') {
            for (const t of c.children) {
              if (t.node.type === 'text') title.push({ text: t.node.text, style: resolveRunStyle(t.modifiers, defaults) });
            }
          } else if (c.node.type === 'fold_content') {
            content = convertChildren(c);
          }
        }
        return v.fold(title, content, ctx);
      }
      default: {
        return undefined;
      }
    }
  }
}
