import { faker } from '@faker-js/faker';
import { eq } from 'drizzle-orm';
import { DEFAULT_FONT_FAMILIES, defaultValues } from '@/const';
import { createDbId, db, FontFamilies, Fonts, TableCode } from '@/db';
import { PostLayoutMode } from '@/enums';
import type { JSONContent } from '@tiptap/core';
import type { Dayjs } from 'dayjs';
import type { PageLayout } from '@/db/schemas/json';

const ROOT_ID = '00000000000000000000000000000000';

type LoroStyle =
  | { type: 'font_weight'; weight: number }
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'strikethrough' }
  | { type: 'underline' }
  | { type: 'text_color'; color: string }
  | { type: 'background_color'; color: string }
  | { type: 'font_family'; family: string }
  | { type: 'font_size'; size: number }
  | { type: 'letter_spacing'; spacing: number };

type LoroAnnotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

type TextSegment = {
  text: string;
  styles?: LoroStyle[];
  annotations?: LoroAnnotation[];
};

type LoroNode = {
  type: string;
  children: string[];
  parent?: string;
  [key: string]: unknown;
};

type DocumentJson = {
  settings: {
    block_gap: number;
    paragraph_indent: number;
    layout_mode:
      | { type: 'continuous'; max_width: number }
      | {
          type: 'paginated';
          page_width: number;
          page_height: number;
          page_margin_top: number;
          page_margin_bottom: number;
          page_margin_left: number;
          page_margin_right: number;
        };
  };
  nodes: Record<string, LoroNode>;
};

type ArchivedNodeEntry = {
  id: string;
  content: string;
};

const generateNodeId = () => faker.string.uuid().replaceAll('-', '');

const MM_TO_PX = 96 / 25.4;
const PX_TO_PT = 72 / 96;

const DEFAULT_STYLES: LoroStyle[] = [
  { type: 'font_family', family: defaultValues.fontFamily },
  { type: 'font_size', size: defaultValues.fontSize },
  { type: 'font_weight', weight: defaultValues.fontWeight },
  { type: 'text_color', color: defaultValues.textColor },
  { type: 'background_color', color: defaultValues.backgroundColor },
  { type: 'letter_spacing', spacing: defaultValues.letterSpacing },
];

function fillDefaultStyles(styles: LoroStyle[]): LoroStyle[] {
  const presentTypes = new Set(styles.map((s) => s.type));
  const filled = [...styles];
  for (const def of DEFAULT_STYLES) {
    if (!presentTypes.has(def.type)) {
      filled.push(def);
    }
  }
  return filled;
}

const defaultFamilyNames = new Set(DEFAULT_FONT_FAMILIES.map((f) => f.familyName));

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function normalizeRatios(widths: number[]): number[] | null {
  if (widths.length === 0) return null;

  const safe = widths.map((width) => (isFiniteNumber(width) && width > 0 ? width : 0));
  const total = safe.reduce((sum, width) => sum + width, 0);
  if (total <= 0) return null;

  const ratios = safe.map((width) => width / total);
  const ratioSum = ratios.reduce((sum, value) => sum + value, 0);
  if (ratioSum <= 0) return null;

  return ratios.map((value) => value / ratioSum);
}

function resolveTableColRatios(rawWidths: (number | undefined)[], contentWidth: number): number[] | null {
  const colCount = rawWidths.length;
  if (colCount === 0) return null;

  const sanitized = rawWidths.map((width) => (isFiniteNumber(width) && width > 0 ? width : undefined));
  const knownWidths = sanitized.filter((width): width is number => width != null);
  const totalKnown = knownWidths.reduce((sum, width) => sum + width, 0);
  const remainingCount = colCount - knownWidths.length;
  const safeContentWidth = isFiniteNumber(contentWidth) ? Math.max(0, contentWidth) : 0;

  let widths: number[];

  if (remainingCount > 0) {
    const remainingWidth = safeContentWidth - totalKnown;
    let fallbackWidth = remainingCount > 0 ? remainingWidth / remainingCount : 0;
    if (!isFiniteNumber(fallbackWidth) || fallbackWidth <= 0) {
      if (safeContentWidth > 0) {
        fallbackWidth = safeContentWidth / colCount;
      } else if (totalKnown > 0) {
        fallbackWidth = totalKnown / colCount;
      } else {
        fallbackWidth = 1;
      }
    }

    widths = sanitized.map((width) => width ?? fallbackWidth);
  } else {
    widths = sanitized.map((width) => width ?? 0);
  }

  return normalizeRatios(widths);
}

function findClosestWeight(target: number, weights: number[]): number {
  let closest = weights[0];
  let minDist = Math.abs(target - closest);
  for (const w of weights) {
    const dist = Math.abs(target - w);
    if (dist < minDist || (dist === minDist && w > closest)) {
      closest = w;
      minDist = dist;
    }
  }
  return closest;
}

async function getAvailableWeights(familyName: string, familyId: string | null): Promise<number[]> {
  const defaultFamily = DEFAULT_FONT_FAMILIES.find((f) => f.familyName === familyName);
  if (defaultFamily) {
    return defaultFamily.fonts.map((f) => f.weight);
  }

  if (!familyId) return [];

  const fonts = await db.select({ weight: Fonts.weight }).from(Fonts).where(eq(Fonts.familyId, familyId));
  return fonts.map((f) => f.weight);
}

async function convertPmMarks(pmMarks: JSONContent['marks']): Promise<{ styles: LoroStyle[]; annotations: LoroAnnotation[] }> {
  const styles: LoroStyle[] = [];
  const annotations: LoroAnnotation[] = [];

  if (!pmMarks) return { styles, annotations };

  let resolvedFamilyName: string | null = null;
  let resolvedFamilyId: string | null = null;

  for (const pmMark of pmMarks) {
    switch (pmMark.type) {
      case 'bold': {
        styles.push({ type: 'font_weight', weight: 700 });
        break;
      }
      case 'italic': {
        styles.push({ type: 'italic' });
        break;
      }
      case 'strike': {
        styles.push({ type: 'strikethrough' });
        break;
      }
      case 'underline': {
        styles.push({ type: 'underline' });
        break;
      }
      case 'link': {
        if (pmMark.attrs?.href) {
          annotations.push({ type: 'link', href: pmMark.attrs.href as string });
        }
        break;
      }
      case 'ruby': {
        if (pmMark.attrs?.text) {
          annotations.push({ type: 'ruby', text: pmMark.attrs.text as string });
        }
        break;
      }
      case 'text_style': {
        if (pmMark.attrs?.textColor) {
          styles.push({ type: 'text_color', color: pmMark.attrs.textColor as string });
        }
        if (pmMark.attrs?.textBackgroundColor) {
          styles.push({ type: 'background_color', color: pmMark.attrs.textBackgroundColor as string });
        }
        if (pmMark.attrs?.fontFamily) {
          const fontFamily = pmMark.attrs.fontFamily as string;
          if (defaultFamilyNames.has(fontFamily)) {
            resolvedFamilyName = fontFamily;
            styles.push({ type: 'font_family', family: fontFamily });
          } else if (fontFamily.startsWith('FONT0')) {
            const result = await db
              .select({ familyName: FontFamilies.familyName, familyId: Fonts.familyId })
              .from(Fonts)
              .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
              .where(eq(Fonts.id, fontFamily))
              .then((r) => r[0]);
            if (result) {
              resolvedFamilyName = result.familyName;
              resolvedFamilyId = result.familyId;
              styles.push({ type: 'font_family', family: result.familyName });
            }
          } else if (fontFamily.startsWith('FNTF0')) {
            const result = await db
              .select({ familyName: FontFamilies.familyName })
              .from(FontFamilies)
              .where(eq(FontFamilies.id, fontFamily))
              .then((r) => r[0]);
            if (result) {
              resolvedFamilyName = result.familyName;
              resolvedFamilyId = fontFamily;
              styles.push({ type: 'font_family', family: result.familyName });
            }
          }
        }
        if (pmMark.attrs?.fontSize) {
          styles.push({ type: 'font_size', size: Math.round(Number(pmMark.attrs.fontSize) * PX_TO_PT * 100) });
        }
        if (pmMark.attrs?.fontWeight) {
          styles.push({ type: 'font_weight', weight: Number(pmMark.attrs.fontWeight) });
        }
        break;
      }
    }
  }

  if (resolvedFamilyName) {
    const availableWeights = await getAvailableWeights(resolvedFamilyName, resolvedFamilyId);

    if (availableWeights.length > 0) {
      const weightStyleIndex = styles.findIndex((s) => s.type === 'font_weight');
      const currentWeight =
        weightStyleIndex === -1 ? defaultValues.fontWeight : (styles[weightStyleIndex] as { type: 'font_weight'; weight: number }).weight;

      if (!availableWeights.includes(currentWeight)) {
        let newWeight: number;
        let addBold = false;

        if (currentWeight >= 700 && availableWeights.length === 1) {
          newWeight = availableWeights[0];
          addBold = true;
        } else if (currentWeight >= 700) {
          newWeight = findClosestWeight(700, availableWeights);
        } else {
          newWeight = findClosestWeight(currentWeight, availableWeights);
        }

        if (weightStyleIndex === -1) {
          styles.push({ type: 'font_weight', weight: newWeight });
        } else {
          styles[weightStyleIndex] = { type: 'font_weight', weight: newWeight };
        }

        if (addBold) {
          styles.push({ type: 'bold' });
        }
      }
    }
  }

  return { styles, annotations };
}

async function mergeInlineContent(content: JSONContent[] | undefined, extraStyles: LoroStyle[]): Promise<TextSegment[]> {
  if (!content || content.length === 0) return [];

  const segments: TextSegment[] = [];

  for (const inline of content) {
    if (inline.type === 'text' && inline.text) {
      const converted = await convertPmMarks(inline.marks);
      const styles = fillDefaultStyles([...converted.styles, ...extraStyles]);
      const segment: TextSegment = { text: inline.text, styles };
      if (converted.annotations.length > 0) segment.annotations = converted.annotations;
      segments.push(segment);
    }
  }

  return segments;
}

function extractTextContent(node: JSONContent): string {
  if (node.type === 'text') return node.text ?? '';
  return (node.content ?? []).map(extractTextContent).join('');
}

function escapeHtml(text: string): string {
  return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}

function wrapWithMarks(html: string, marks?: JSONContent['marks']): string {
  if (!marks || marks.length === 0) return html;

  let result = html;
  for (const mark of marks) {
    switch (mark.type) {
      case 'bold': {
        result = `<strong>${result}</strong>`;
        break;
      }
      case 'italic': {
        result = `<em>${result}</em>`;
        break;
      }
      case 'strike': {
        result = `<del>${result}</del>`;
        break;
      }
      case 'underline': {
        result = `<u>${result}</u>`;
        break;
      }
      case 'link': {
        result = `<a href="${escapeHtml(mark.attrs?.href ?? '')}">${result}</a>`;
        break;
      }
      case 'ruby': {
        result = `<ruby>${result}<rp>(</rp><rt>${escapeHtml(mark.attrs?.text ?? '')}</rt><rp>)</rp></ruby>`;
        break;
      }
      case 'text_style': {
        const styles: string[] = [];
        if (mark.attrs?.textColor) styles.push(`color:${mark.attrs.textColor}`);
        if (mark.attrs?.textBackgroundColor) styles.push(`background-color:${mark.attrs.textBackgroundColor}`);
        if (mark.attrs?.fontFamily) styles.push(`font-family:${mark.attrs.fontFamily}`);
        if (mark.attrs?.fontSize) styles.push(`font-size:${mark.attrs.fontSize}px`);
        if (mark.attrs?.fontWeight) styles.push(`font-weight:${mark.attrs.fontWeight}`);
        if (styles.length > 0) result = `<span style="${styles.join(';')}">${result}</span>`;
        break;
      }
    }
  }
  return result;
}

function nodeAttrs(node: JSONContent): string {
  let attrs = ` data-node-type="${escapeHtml(node.type ?? '')}"`;
  if (node.attrs?.nodeId) attrs += ` data-node-id="${escapeHtml(String(node.attrs.nodeId))}"`;
  return attrs;
}

function jsonContentToHtml(node: JSONContent): string {
  const children = (node.content ?? []).map(jsonContentToHtml).join('');
  const da = nodeAttrs(node);

  switch (node.type) {
    case 'text': {
      return wrapWithMarks(escapeHtml(node.text ?? ''), node.marks);
    }
    case 'paragraph': {
      const pStyles: string[] = [];
      if (node.attrs?.textAlign && node.attrs.textAlign !== 'left') pStyles.push(`text-align:${node.attrs.textAlign}`);
      if (node.attrs?.lineHeight && node.attrs.lineHeight !== 1.6) pStyles.push(`line-height:${node.attrs.lineHeight}`);
      if (node.attrs?.letterSpacing && node.attrs.letterSpacing !== 0) pStyles.push(`letter-spacing:${node.attrs.letterSpacing}em`);
      const pStyleAttr = pStyles.length > 0 ? ` style="${pStyles.join(';')}"` : '';
      return `<p${da}${pStyleAttr}>${children}</p>`;
    }
    case 'hard_break': {
      return `<br${da}>`;
    }
    case 'bullet_list': {
      return `<ul${da}>${children}</ul>`;
    }
    case 'ordered_list': {
      const olStart = node.attrs?.start;
      const olStartAttr = olStart && olStart !== 1 ? ` start="${olStart}"` : '';
      return `<ol${da}${olStartAttr}>${children}</ol>`;
    }
    case 'list_item': {
      return `<li${da}>${children}</li>`;
    }
    case 'blockquote': {
      return `<blockquote${da} data-type="${escapeHtml(String(node.attrs?.type ?? 'left-line'))}">${children}</blockquote>`;
    }
    case 'callout': {
      return `<div${da} data-type="${escapeHtml(String(node.attrs?.type ?? 'info'))}">${children}</div>`;
    }
    case 'code_block': {
      return `<pre${da}><code data-lang="${escapeHtml(String(node.attrs?.language ?? 'text'))}">${children}</code></pre>`;
    }
    case 'html_block': {
      return `<pre${da}>${children}</pre>`;
    }
    case 'image': {
      const imgAttrs: string[] = [];
      if (node.attrs?.id) imgAttrs.push(`data-id="${escapeHtml(String(node.attrs.id))}"`);
      if (node.attrs?.url) imgAttrs.push(`src="${escapeHtml(String(node.attrs.url))}"`);
      if (node.attrs?.ratio) imgAttrs.push(`data-ratio="${escapeHtml(String(node.attrs.ratio))}"`);
      if (node.attrs?.proportion) imgAttrs.push(`data-proportion="${escapeHtml(String(node.attrs.proportion))}"`);
      if (node.attrs?.placeholder) imgAttrs.push(`data-placeholder="${escapeHtml(String(node.attrs.placeholder))}"`);
      if (node.attrs?.size) imgAttrs.push(`data-size="${escapeHtml(String(node.attrs.size))}"`);
      return `<img${da} ${imgAttrs.join(' ')}>`;
    }
    case 'file': {
      const fileUrl = escapeHtml(String(node.attrs?.url ?? ''));
      const fileName = escapeHtml(String(node.attrs?.name ?? ''));
      const fileAttrs: string[] = [];
      if (node.attrs?.id) fileAttrs.push(`data-id="${escapeHtml(String(node.attrs.id))}"`);
      if (node.attrs?.size) fileAttrs.push(`data-size="${escapeHtml(String(node.attrs.size))}"`);
      return `<a${da} href="${fileUrl}" ${fileAttrs.join(' ')}>${fileName}</a>`;
    }
    case 'embed': {
      const embedUrl = escapeHtml(String(node.attrs?.url ?? ''));
      const embedTitle = escapeHtml(String(node.attrs?.title ?? node.attrs?.url ?? ''));
      const embedAttrs: string[] = [];
      if (node.attrs?.id) embedAttrs.push(`data-id="${escapeHtml(String(node.attrs.id))}"`);
      if (node.attrs?.description) embedAttrs.push(`data-description="${escapeHtml(String(node.attrs.description))}"`);
      if (node.attrs?.thumbnailUrl) embedAttrs.push(`data-thumbnail-url="${escapeHtml(String(node.attrs.thumbnailUrl))}"`);
      if (node.attrs?.html) embedAttrs.push(`data-html="${escapeHtml(String(node.attrs.html))}"`);
      if (node.attrs?.proportion) embedAttrs.push(`data-proportion="${escapeHtml(String(node.attrs.proportion))}"`);
      return `<a${da} href="${embedUrl}" ${embedAttrs.join(' ')}>${embedTitle}</a>`;
    }
    case 'table': {
      const tableAttrs: string[] = [];
      if (node.attrs?.borderStyle && node.attrs.borderStyle !== 'solid') {
        tableAttrs.push(`data-border-style="${escapeHtml(String(node.attrs.borderStyle))}"`);
      }
      const tableAttrStr = tableAttrs.length > 0 ? ` ${tableAttrs.join(' ')}` : '';
      return `<table${da}${tableAttrStr}>${children}</table>`;
    }
    case 'table_row': {
      return `<tr${da}>${children}</tr>`;
    }
    case 'table_cell': {
      const tdAttrs: string[] = [];
      if (node.attrs?.colspan && node.attrs.colspan !== 1) tdAttrs.push(`colspan="${node.attrs.colspan}"`);
      if (node.attrs?.rowspan && node.attrs.rowspan !== 1) tdAttrs.push(`rowspan="${node.attrs.rowspan}"`);
      if (node.attrs?.colwidth) tdAttrs.push(`data-colwidth="${escapeHtml(String(node.attrs.colwidth))}"`);
      const tdAttrStr = tdAttrs.length > 0 ? ` ${tdAttrs.join(' ')}` : '';
      return `<td${da}${tdAttrStr}>${children}</td>`;
    }
    case 'horizontal_rule': {
      return `<hr${da} data-type="${escapeHtml(String(node.attrs?.type ?? 'light-line'))}">`;
    }
    case 'fold': {
      const foldOpen = node.attrs?.open === false ? '' : ' open';
      return `<details${da}${foldOpen}><summary>${escapeHtml(String(node.attrs?.title ?? ''))}</summary>${children}</details>`;
    }
    default: {
      return children;
    }
  }
}

async function convertNode(
  pmNode: JSONContent,
  parentId: string,
  nodes: Record<string, LoroNode>,
  archivedNodes: ArchivedNodeEntry[],
  contentWidth: number,
  nodeIdMap: Map<string, { loroId: string; excerpt: string }>,
): Promise<string | null> {
  const nodeId = generateNodeId();

  const pmNodeId = pmNode.attrs?.nodeId as string | undefined;
  if (pmNodeId) {
    const text = extractTextContent(pmNode);
    const excerpt = text ? (text.length > 20 ? text.slice(0, 20) + '...' : text) : '(내용 없음)';
    nodeIdMap.set(pmNodeId, { loroId: nodeId, excerpt });
  }

  switch (pmNode.type) {
    case 'paragraph': {
      const letterSpacing = pmNode.attrs?.letterSpacing ?? 0;
      const letterSpacingInt = Math.round(letterSpacing * 100);
      const extraStyles: LoroStyle[] = letterSpacingInt === 0 ? [] : [{ type: 'letter_spacing', spacing: letterSpacingInt }];

      const children: string[] = [];
      let pendingInlines: JSONContent[] = [];

      const flushText = async () => {
        const segments = await mergeInlineContent(pendingInlines, extraStyles);
        if (segments.length > 0) {
          const textNodeId = generateNodeId();
          nodes[textNodeId] = {
            type: 'text',
            text: segments,
            children: [],
            parent: nodeId,
          } as LoroNode;
          children.push(textNodeId);
        }
        pendingInlines = [];
      };

      for (const inline of pmNode.content ?? []) {
        if (inline.type === 'hard_break') {
          await flushText();
          const hardBreakId = generateNodeId();
          nodes[hardBreakId] = {
            type: 'hard_break',
            children: [],
            parent: nodeId,
          };
          children.push(hardBreakId);
        } else {
          pendingInlines.push(inline);
        }
      }

      await flushText();

      nodes[nodeId] = {
        type: 'paragraph',
        align: pmNode.attrs?.textAlign ?? 'left',
        line_height: Math.round((pmNode.attrs?.lineHeight ?? 1.6) * 100),
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'blockquote': {
      const pmType = pmNode.attrs?.type ?? 'left-line';
      const variantMap: Record<string, string> = {
        'left-line': 'left_line',
        'left-quote': 'left_quote',
        'message-sent': 'message_sent',
        'message-received': 'message_received',
      };
      const variant = variantMap[pmType] ?? 'left_line';

      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      nodes[nodeId] = {
        type: 'blockquote',
        variant,
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'callout': {
      const variant = pmNode.attrs?.type ?? 'info';

      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      nodes[nodeId] = {
        type: 'callout',
        variant,
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'bullet_list': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      nodes[nodeId] = {
        type: 'bullet_list',
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'ordered_list': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      nodes[nodeId] = {
        type: 'ordered_list',
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'list_item': {
      const paragraphIds: string[] = [];
      const otherIds: string[] = [];

      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) {
            if (nodes[childId].type === 'paragraph') {
              paragraphIds.push(childId);
            } else {
              otherIds.push(childId);
            }
          }
        }
      }

      if (paragraphIds.length > 1) {
        const firstId = paragraphIds[0];
        const firstNode = nodes[firstId];
        for (let i = 1; i < paragraphIds.length; i++) {
          const extraId = paragraphIds[i];
          const extraNode = nodes[extraId];

          if (extraNode.children.length > 0) {
            const hardBreakId = generateNodeId();
            nodes[hardBreakId] = {
              type: 'hard_break',
              children: [],
              parent: firstId,
            };
            firstNode.children.push(hardBreakId);

            for (const inlineId of extraNode.children) {
              nodes[inlineId].parent = firstId;
              firstNode.children.push(inlineId);
            }
          }

          // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
          delete nodes[extraId];
        }
      }

      const children = paragraphIds.length > 0 ? [paragraphIds[0], ...otherIds] : otherIds;

      nodes[nodeId] = {
        type: 'list_item',
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'image': {
      nodes[nodeId] = {
        type: 'image',
        id: pmNode.attrs?.id ?? null,
        proportion: pmNode.attrs?.proportion ?? 1,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'file': {
      nodes[nodeId] = {
        type: 'file',
        id: pmNode.attrs?.id ?? null,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'embed': {
      nodes[nodeId] = {
        type: 'embed',
        id: pmNode.attrs?.id ?? null,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'table': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      const borderStyleMap: Record<string, string> = {
        solid: 'solid',
        dashed: 'dashed',
        dotted: 'dotted',
        none: 'none',
      };

      const loroNode: LoroNode = {
        type: 'table',
        border_style: borderStyleMap[pmNode.attrs?.borderStyle as string] ?? 'solid',
        align: 'left',
        children,
        parent: parentId,
        proportion: 1,
      };

      const firstRow = pmNode.content?.find((child) => child.type === 'table_row');
      const firstRowCells = firstRow?.content?.filter((child) => child.type === 'table_cell') ?? [];
      const rawWidths = firstRowCells.map((cell) => {
        const colwidth = cell.attrs?.colwidth;
        return Array.isArray(colwidth) ? colwidth[0] : undefined;
      });

      const explicitWidths = rawWidths.map((width) => (isFiniteNumber(width) && width > 0 ? width : undefined));
      const hasAnyExplicit = explicitWidths.some((width) => width != null);
      const hasAllExplicit = explicitWidths.length > 0 && explicitWidths.every((width) => width != null);

      if (hasAllExplicit) {
        const totalWidth = explicitWidths.reduce((sum, width) => sum + (width ?? 0), 0);
        if (isFiniteNumber(contentWidth) && contentWidth > 0 && isFiniteNumber(totalWidth) && totalWidth > 0) {
          loroNode.proportion = Math.min(1, Math.max(0, totalWidth / contentWidth));
        }
      }

      let ratios: number[] | null = null;

      if (hasAnyExplicit) {
        ratios = resolveTableColRatios(rawWidths, contentWidth);
        if (!ratios && hasAllExplicit) {
          ratios = normalizeRatios(explicitWidths.map((width) => width ?? 0));
        }
      }

      if (ratios) {
        for (const rowId of children) {
          const rowNode = nodes[rowId];
          if (!rowNode || rowNode.type !== 'table_row' || !Array.isArray(rowNode.children)) {
            continue;
          }

          for (const [index, cellId] of rowNode.children.entries()) {
            const ratio = ratios[index];
            if (ratio == null || typeof cellId !== 'string') {
              continue;
            }

            const cellNode = nodes[cellId];
            if (!cellNode || cellNode.type !== 'table_cell') {
              continue;
            }

            cellNode.col_width = ratio;
          }
        }
      }

      nodes[nodeId] = loroNode;
      return nodeId;
    }

    case 'table_row': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) children.push(childId);
        }
      }

      nodes[nodeId] = {
        type: 'table_row',
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'table_cell': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          if (child.type === 'table') {
            const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
            archivedNodes.push({ id: archivedId, content: jsonContentToHtml(child) });
            const nestedId = generateNodeId();
            nodes[nestedId] = {
              type: 'archived',
              id: archivedId,
              children: [],
              parent: nodeId,
            };
            children.push(nestedId);
          } else {
            const childId = await convertNode(child, nodeId, nodes, archivedNodes, contentWidth, nodeIdMap);
            if (childId) children.push(childId);
          }
        }
      }

      const loroNode: LoroNode = {
        type: 'table_cell',
        children,
        parent: parentId,
      };

      nodes[nodeId] = loroNode;
      return nodeId;
    }

    case 'horizontal_rule': {
      const pmType = pmNode.attrs?.type ?? 'light-line';
      const variantMap: Record<string, string> = {
        'light-line': 'line',
        'dashed-line': 'dashed_line',
        'circle-line': 'circle_line',
        'diamond-line': 'diamond_line',
        circle: 'circle',
        diamond: 'diamond',
        'three-circles': 'three_circles',
        'three-diamonds': 'three_diamonds',
        zigzag: 'zigzag',
      };

      nodes[nodeId] = {
        type: 'horizontal_rule',
        variant: variantMap[pmType] ?? 'line',
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'page_break': {
      return null;
    }

    case 'fold': {
      const foldTitleId = generateNodeId();
      const foldContentId = generateNodeId();
      const title = (pmNode.attrs?.title as string) ?? '';

      const titleChildren: string[] = [];
      if (title) {
        const titleTextId = generateNodeId();
        nodes[titleTextId] = {
          type: 'text',
          text: [{ text: title }],
          children: [],
          parent: foldTitleId,
        } as LoroNode;
        titleChildren.push(titleTextId);
      }

      nodes[foldTitleId] = {
        type: 'fold_title',
        children: titleChildren,
        parent: nodeId,
      };

      const contentChildren: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = await convertNode(child, foldContentId, nodes, archivedNodes, contentWidth, nodeIdMap);
          if (childId) contentChildren.push(childId);
        }
      }

      nodes[foldContentId] = {
        type: 'fold_content',
        children: contentChildren,
        parent: nodeId,
      };

      nodes[nodeId] = {
        type: 'fold',
        children: [foldTitleId, foldContentId],
        parent: parentId,
      };
      return nodeId;
    }

    case 'code_block': {
      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: jsonContentToHtml(pmNode) });

      nodes[nodeId] = {
        type: 'archived',
        id: archivedId,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'html_block': {
      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: jsonContentToHtml(pmNode) });

      nodes[nodeId] = {
        type: 'archived',
        id: archivedId,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'paywall': {
      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: jsonContentToHtml(pmNode) });

      nodes[nodeId] = {
        type: 'archived',
        id: archivedId,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    default: {
      return null;
    }
  }
}

export async function convertPostToDocumentJson(
  body: JSONContent,
  options: {
    maxWidth: number;
    layoutMode: PostLayoutMode;
    pageLayout: PageLayout | null;
    anchors?: { nodeId: string; name: string | null; createdAt: Dayjs }[];
    userId?: string;
  },
): Promise<{ json: DocumentJson; archivedNodes: ArchivedNodeEntry[] }> {
  const bodyNode = body.content?.[0];
  const paragraphIndent = Math.round((bodyNode?.attrs?.paragraphIndent ?? 1) * 100);
  const blockGap = Math.round((bodyNode?.attrs?.blockGap ?? 1) * 100);

  let layoutMode: DocumentJson['settings']['layout_mode'];

  if (options.layoutMode === PostLayoutMode.PAGE && options.pageLayout) {
    layoutMode = {
      type: 'paginated',
      page_width: options.pageLayout.width * MM_TO_PX,
      page_height: options.pageLayout.height * MM_TO_PX,
      page_margin_top: options.pageLayout.marginTop * MM_TO_PX,
      page_margin_bottom: options.pageLayout.marginBottom * MM_TO_PX,
      page_margin_left: options.pageLayout.marginLeft * MM_TO_PX,
      page_margin_right: options.pageLayout.marginRight * MM_TO_PX,
    };
  } else {
    layoutMode = {
      type: 'continuous',
      max_width: Math.min(options.maxWidth, 800),
    };
  }

  const contentWidth =
    layoutMode.type === 'paginated'
      ? Math.max(0, layoutMode.page_width - layoutMode.page_margin_left - layoutMode.page_margin_right)
      : Math.max(0, layoutMode.max_width);

  const nodes: Record<string, LoroNode> = {};
  const archivedNodes: ArchivedNodeEntry[] = [];
  const nodeIdMap = new Map<string, { loroId: string; excerpt: string }>();

  const rootChildren: string[] = [];
  const blocks = bodyNode?.content ?? [];

  for (const block of blocks) {
    if (block.type === 'page_break') {
      const lastParagraphId = [...rootChildren].toReversed().find((id) => nodes[id]?.type === 'paragraph');
      if (lastParagraphId) {
        const pageBreakId = generateNodeId();
        nodes[pageBreakId] = {
          type: 'page_break',
          children: [],
          parent: lastParagraphId,
        };
        nodes[lastParagraphId].children.push(pageBreakId);
      }
      continue;
    }

    const childId = await convertNode(block, ROOT_ID, nodes, archivedNodes, contentWidth, nodeIdMap);
    if (childId) rootChildren.push(childId);
  }

  const lastRootChildId = rootChildren.at(-1);
  if (!lastRootChildId || nodes[lastRootChildId]?.type !== 'paragraph') {
    const trailingParagraphId = generateNodeId();
    nodes[trailingParagraphId] = {
      type: 'paragraph',
      align: 'left',
      line_height: 160,
      children: [],
      parent: ROOT_ID,
    };
    rootChildren.push(trailingParagraphId);
  }

  nodes[ROOT_ID] = {
    type: 'root',
    children: rootChildren,
    cascade_attrs: {
      'style:font_family': defaultValues.fontFamily,
      'style:font_size': defaultValues.fontSize,
      'style:font_weight': defaultValues.fontWeight,
      'style:text_color': defaultValues.textColor,
      'style:background_color': defaultValues.backgroundColor,
      'style:letter_spacing': defaultValues.letterSpacing,
      'paragraph:line_height': defaultValues.lineHeight,
    },
  };

  if (options.anchors && options.anchors.length > 0 && options.userId) {
    const anchorMap = new Map(options.anchors.map((a) => [a.nodeId, a]));
    for (const [pmId, { loroId, excerpt }] of nodeIdMap) {
      const anchor = anchorMap.get(pmId);
      if (anchor && nodes[loroId]) {
        const remarkId = generateNodeId();
        nodes[loroId].remarks = {
          [remarkId]: {
            user_id: options.userId,
            text: anchor.name || excerpt,
            created_at: anchor.createdAt.valueOf(),
          },
        };
      }
    }
  }

  const json: DocumentJson = {
    settings: {
      block_gap: blockGap,
      paragraph_indent: paragraphIndent,
      layout_mode: layoutMode,
    },
    nodes,
  };

  return { json, archivedNodes };
}
