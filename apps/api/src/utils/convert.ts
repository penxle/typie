import { faker } from '@faker-js/faker';
import { Node } from '@tiptap/pm/model';
import { createDbId, TableCode } from '@/db';
import { PostLayoutMode } from '@/enums';
import { schema } from '@/pm';
import type { JSONContent } from '@tiptap/core';
import type { PageLayout } from '@/db/schemas/json';

const ROOT_ID = '00000000000000000000000000000000';

type LoroStyle =
  | { type: 'font_weight'; weight: number }
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
  annotation_ids?: string[];
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
  styles: Record<string, unknown>;
  nodes: Record<string, LoroNode>;
  annotations: Record<string, unknown>;
};

type ArchivedNodeEntry = {
  id: string;
  content: string;
};

const generateNodeId = () => faker.string.uuid().replaceAll('-', '');

const MM_TO_PX = 96 / 25.4;
const PX_TO_PT = 72 / 96;

function convertPmMarks(
  pmMarks: JSONContent['marks'],
  annotations: Record<string, LoroAnnotation>,
): { styles: LoroStyle[]; annotationIds: string[] } {
  const styles: LoroStyle[] = [];
  const annotationIds: string[] = [];

  if (!pmMarks) return { styles, annotationIds };

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
          const id = generateNodeId();
          annotations[id] = { type: 'link', href: pmMark.attrs.href as string };
          annotationIds.push(id);
        }
        break;
      }
      case 'ruby': {
        if (pmMark.attrs?.text) {
          const id = generateNodeId();
          annotations[id] = { type: 'ruby', text: pmMark.attrs.text as string };
          annotationIds.push(id);
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
          styles.push({ type: 'font_family', family: pmMark.attrs.fontFamily as string });
        }
        if (pmMark.attrs?.fontSize) {
          styles.push({ type: 'font_size', size: Number(pmMark.attrs.fontSize) * PX_TO_PT });
        }
        if (pmMark.attrs?.fontWeight) {
          styles.push({ type: 'font_weight', weight: Number(pmMark.attrs.fontWeight) });
        }
        break;
      }
    }
  }

  return { styles, annotationIds };
}

function mergeInlineContent(
  content: JSONContent[] | undefined,
  extraStyles: LoroStyle[],
  annotations: Record<string, LoroAnnotation>,
): TextSegment[] {
  if (!content || content.length === 0) return [];

  const segments: TextSegment[] = [];

  for (const inline of content) {
    if (inline.type === 'text' && inline.text) {
      const converted = convertPmMarks(inline.marks, annotations);
      const styles = [...converted.styles, ...extraStyles];
      const segment: TextSegment = { text: inline.text };
      if (styles.length > 0) segment.styles = styles;
      if (converted.annotationIds.length > 0) segment.annotation_ids = converted.annotationIds;
      segments.push(segment);
    } else if (inline.type === 'hard_break') {
      const segment: TextSegment = { text: '\n' };
      if (extraStyles.length > 0) segment.styles = [...extraStyles];
      segments.push(segment);
    }
  }

  return segments;
}

function convertNode(
  pmNode: JSONContent,
  parentId: string,
  nodes: Record<string, LoroNode>,
  archivedNodes: ArchivedNodeEntry[],
  annotations: Record<string, LoroAnnotation>,
): string | null {
  const nodeId = generateNodeId();

  switch (pmNode.type) {
    case 'paragraph': {
      const letterSpacing = pmNode.attrs?.letterSpacing ?? 0;
      const extraStyles: LoroStyle[] = letterSpacing === 0 ? [] : [{ type: 'letter_spacing', spacing: letterSpacing }];

      const children: string[] = [];
      const segments = mergeInlineContent(pmNode.content, extraStyles, annotations);

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

      nodes[nodeId] = {
        type: 'paragraph',
        align: pmNode.attrs?.textAlign ?? 'left',
        line_height: pmNode.attrs?.lineHeight ?? 1.6,
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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
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
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
          if (childId) children.push(childId);
        }
      }

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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
          if (childId) children.push(childId);
        }
      }

      const borderStyleMap: Record<string, string> = {
        solid: 'solid',
        dashed: 'dashed',
        dotted: 'dotted',
        none: 'none',
      };

      nodes[nodeId] = {
        type: 'table',
        border_style: borderStyleMap[pmNode.attrs?.borderStyle as string] ?? 'solid',
        children,
        parent: parentId,
      };
      return nodeId;
    }

    case 'table_row': {
      const children: string[] = [];
      if (pmNode.content) {
        for (const child of pmNode.content) {
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
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
          const childId = convertNode(child, nodeId, nodes, archivedNodes, annotations);
          if (childId) children.push(childId);
        }
      }

      const colwidth = pmNode.attrs?.colwidth;
      const colWidth = Array.isArray(colwidth) ? (colwidth[0] as number | undefined) : undefined;

      const loroNode: LoroNode = {
        type: 'table_cell',
        children,
        parent: parentId,
      };

      if (colWidth != null) {
        loroNode.col_width = colWidth;
      }

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

    case 'hard_break': {
      nodes[nodeId] = {
        type: 'hard_break',
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'page_break': {
      nodes[nodeId] = {
        type: 'page_break',
        children: [],
        parent: parentId,
      };
      return nodeId;
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
          const childId = convertNode(child, foldContentId, nodes, archivedNodes, annotations);
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
      const textContent = (pmNode.content ?? []).map((c) => c.text ?? '').join('');

      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: textContent });

      nodes[nodeId] = {
        type: 'archived',
        id: archivedId,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'html_block': {
      const textContent = (pmNode.content ?? []).map((c) => c.text ?? '').join('');

      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: textContent });

      nodes[nodeId] = {
        type: 'archived',
        id: archivedId,
        children: [],
        parent: parentId,
      };
      return nodeId;
    }

    case 'paywall': {
      const pmNodeModel = Node.fromJSON(schema, pmNode);
      const textContent = pmNodeModel.textContent;

      const archivedId = createDbId(TableCode.DOCUMENT_ARCHIVED_NODES);
      archivedNodes.push({ id: archivedId, content: textContent });

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

export function convertPostToDocumentJson(
  body: JSONContent,
  options: {
    maxWidth: number;
    layoutMode: PostLayoutMode;
    pageLayout: PageLayout | null;
  },
): { json: DocumentJson; archivedNodes: ArchivedNodeEntry[] } {
  const bodyNode = body.content?.[0];
  const paragraphIndent = bodyNode?.attrs?.paragraphIndent ?? 1;
  const blockGap = bodyNode?.attrs?.blockGap ?? 1;

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

  const nodes: Record<string, LoroNode> = {};
  const annotations: Record<string, LoroAnnotation> = {};
  const archivedNodes: ArchivedNodeEntry[] = [];

  const rootChildren: string[] = [];
  const blocks = bodyNode?.content ?? [];

  for (const block of blocks) {
    const childId = convertNode(block, ROOT_ID, nodes, archivedNodes, annotations);
    if (childId) rootChildren.push(childId);
  }

  if (rootChildren.length === 0) {
    const emptyParagraphId = generateNodeId();
    nodes[emptyParagraphId] = {
      type: 'paragraph',
      align: 'left',
      line_height: 1.6,
      children: [],
      parent: ROOT_ID,
    };
    rootChildren.push(emptyParagraphId);
  }

  nodes[ROOT_ID] = {
    type: 'root',
    children: rootChildren,
  };

  const json: DocumentJson = {
    settings: {
      block_gap: blockGap,
      paragraph_indent: paragraphIndent,
      layout_mode: layoutMode,
    },
    styles: {
      font_family: 'Pretendard',
      font_size: 12,
      font_weight: 400,
      text_color: 'black',
      background_color: 'none',
      letter_spacing: 0,
      italic: false,
      strikethrough: false,
      underline: false,
    },
    nodes,
    annotations,
  };

  return { json, archivedNodes };
}
