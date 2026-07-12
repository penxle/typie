import type { Modifier, ModifierType, PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/server';

const ROOT_ID = '0'.repeat(32);

export type LegacyRemark = { id: string; user_id: string; text: string; created_at: number };

export type LegacySegment = {
  text: string;
  styles?: LegacyStyle[];
  annotations?: LegacyAnnotation[];
};

export type LegacyStyle =
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'underline' }
  | { type: 'strikethrough' }
  | { type: 'font_size'; size: number }
  | { type: 'font_family'; family: string }
  | { type: 'font_weight'; weight: number }
  | { type: 'text_color'; color: string }
  | { type: 'background_color'; color: string }
  | { type: 'letter_spacing'; spacing: number };

export type LegacyAnnotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

export type LegacyNodeEntry = {
  type: string;
  children?: string[];
  parent?: string;
  cascade_attrs?: Record<string, unknown>;
  remarks?: Record<string, { user_id: string; text: string; created_at: number }>;
  text?: LegacySegment[];
  [key: string]: unknown;
};

export type LegacyLayoutMode =
  | {
      type: 'paginated';
      page_width: number;
      page_height: number;
      page_margin_top: number;
      page_margin_bottom: number;
      page_margin_left: number;
      page_margin_right: number;
    }
  | { type: 'continuous'; max_width: number };

export type LegacyDocumentJson = {
  settings: {
    block_gap?: number;
    paragraph_indent?: number;
    layout_mode?: LegacyLayoutMode;
  };
  nodes: Record<string, LegacyNodeEntry>;
};

export type RemarkAnchor = { path: number[]; nodeId: string; remarks: LegacyRemark[] };

export type ConvertResult = { plain: PlainDoc; remarkAnchors: RemarkAnchor[]; warnings: string[] };

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Math.round(v)));

const toProportion = (raw: unknown): number | undefined => (raw == null ? undefined : clamp(Number(raw) * 100, 0, 100));

const toColWidth = (raw: unknown): number | undefined => (raw == null ? undefined : Math.max(0, Math.round(Number(raw) * 100)));

const MODIFIER_TYPE_ORDER: ModifierType[] = [
  'bold',
  'italic',
  'underline',
  'strikethrough',
  'font_size',
  'font_family',
  'font_weight',
  'text_color',
  'background_color',
  'letter_spacing',
  'link',
  'ruby',
  'line_height',
  'block_gap',
  'paragraph_indent',
  'alignment',
];

const sortCarry = (modifiers: Modifier[]): Modifier[] =>
  modifiers.toSorted((a, b) => MODIFIER_TYPE_ORDER.indexOf(a.type) - MODIFIER_TYPE_ORDER.indexOf(b.type));

const modifiersSignature = (modifiers: PlainNodeEntry['modifiers']): string =>
  JSON.stringify(Object.entries(modifiers).toSorted(([a], [b]) => a.localeCompare(b)));

const mergeAdjacentTextRuns = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  const out: PlainNodeEntry[] = [];
  for (const child of children) {
    const prev = out.at(-1);
    if (
      prev &&
      prev.node.type === 'text' &&
      child.node.type === 'text' &&
      modifiersSignature(prev.modifiers) === modifiersSignature(child.modifiers)
    ) {
      prev.node.text += child.node.text;
      transferTags(child, prev, ctx);
    } else {
      out.push(child);
    }
  }
  return out;
};

const INLINE_NODE_TYPES = new Set(['text', 'tab', 'hard_break']);

const emptyParagraph = (): PlainNodeEntry => ({
  node: { type: 'paragraph' },
  modifiers: {} as PlainNodeEntry['modifiers'],
  carry: [],
  children: [],
});

const normalizeBlockChildren = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  const out: PlainNodeEntry[] = [];
  let inlineBuffer: PlainNodeEntry[] = [];
  const flushInlines = () => {
    if (inlineBuffer.length > 0) {
      const wrapped = emptyParagraph();
      wrapped.children = mergeAdjacentTextRuns(inlineBuffer, ctx);
      out.push(wrapped);
      inlineBuffer = [];
    }
  };
  for (const child of children) {
    if (INLINE_NODE_TYPES.has(child.node.type)) {
      inlineBuffer.push(child);
      continue;
    }
    flushInlines();
    let block = child;
    if (block.node.type === 'list_item') {
      ctx.warnings.push('orphan list_item wrapped into bullet_list');
      block = makeEntry({ type: 'bullet_list' }, {}, [block]);
    }
    const prev = out.at(-1);
    if (
      prev &&
      (block.node.type === 'bullet_list' || block.node.type === 'ordered_list') &&
      prev.node.type === block.node.type &&
      Object.keys(prev.modifiers).length === 0 &&
      Object.keys(block.modifiers).length === 0
    ) {
      prev.children.push(...block.children);
      transferTags(block, prev, ctx);
      continue;
    }
    out.push(block);
  }
  flushInlines();
  return out;
};

const roundLayoutMode = (layout: LegacyLayoutMode) =>
  layout.type === 'paginated'
    ? {
        type: 'paginated' as const,
        page_width: Math.round(layout.page_width),
        page_height: Math.round(layout.page_height),
        page_margin_top: Math.round(layout.page_margin_top),
        page_margin_bottom: Math.round(layout.page_margin_bottom),
        page_margin_left: Math.round(layout.page_margin_left),
        page_margin_right: Math.round(layout.page_margin_right),
      }
    : { type: 'continuous' as const, max_width: Math.round(layout.max_width) };

type RemarkTag = { nodeId: string; remarks: LegacyRemark[] };

type WalkCtx = {
  nodes: Record<string, LegacyNodeEntry>;
  anchorTags: WeakMap<PlainNodeEntry, RemarkTag[]>;
  orphanTags: RemarkTag[];
  warnings: string[];
  rootEffective: Partial<Record<ModifierType, Modifier>>;
};

const pushTags = (entry: PlainNodeEntry, tags: RemarkTag[], ctx: WalkCtx): void => {
  const existing = ctx.anchorTags.get(entry);
  if (existing) existing.push(...tags);
  else ctx.anchorTags.set(entry, [...tags]);
};

const transferTags = (from: PlainNodeEntry, to: PlainNodeEntry, ctx: WalkCtx): void => {
  const tags = ctx.anchorTags.get(from);
  if (!tags) return;
  ctx.anchorTags.delete(from);
  pushTags(to, tags, ctx);
};

const INHERITABLE_RUN_KINDS = ['font_family', 'font_size', 'font_weight', 'letter_spacing'] as const;

const INHERITABLE_TEXT_DEFAULTS: Partial<Record<ModifierType, Modifier>> = {
  font_family: { type: 'font_family', value: 'Pretendard' },
  font_size: { type: 'font_size', value: 1200 },
  font_weight: { type: 'font_weight', value: 400 },
  letter_spacing: { type: 'letter_spacing', value: 0 },
};

const isDefaultTextColor = (modifier: Modifier): boolean => {
  if (modifier.type !== 'text_color') return false;
  const value = modifier.value.toLowerCase();
  return value === 'black' || value === '#000000';
};

const dropInheritedEquals = (modifiers: Partial<Record<ModifierType, Modifier>>, ctx: WalkCtx): void => {
  if (modifiers.text_color && isDefaultTextColor(modifiers.text_color)) {
    delete modifiers.text_color;
  }
  if (modifiers.font_family && JSON.stringify(modifiers.font_family) === JSON.stringify(ctx.rootEffective.font_family)) {
    delete modifiers.font_family;
  }
  if (modifiers.font_size && JSON.stringify(modifiers.font_size) === JSON.stringify(ctx.rootEffective.font_size)) {
    delete modifiers.font_size;
  }
  if (modifiers.font_weight && JSON.stringify(modifiers.font_weight) === JSON.stringify(ctx.rootEffective.font_weight)) {
    delete modifiers.font_weight;
  }
  if (modifiers.letter_spacing && JSON.stringify(modifiers.letter_spacing) === JSON.stringify(ctx.rootEffective.letter_spacing)) {
    delete modifiers.letter_spacing;
  }
};

const CASCADE_KEYS: Record<string, { type: ModifierType }> = {
  'style:font_family': { type: 'font_family' },
  'style:font_size': { type: 'font_size' },
  'style:font_weight': { type: 'font_weight' },
  'style:letter_spacing': { type: 'letter_spacing' },
  'paragraph:line_height': { type: 'line_height' },
  'style:bold': { type: 'bold' },
  'style:italic': { type: 'italic' },
  'style:underline': { type: 'underline' },
  'style:strikethrough': { type: 'strikethrough' },
  'style:text_color': { type: 'text_color' },
  'style:background_color': { type: 'background_color' },
};

const modifierFromCascade = (key: ModifierType, raw: unknown, warnings: string[]): Modifier | null => {
  switch (key) {
    case 'bold':
    case 'italic':
    case 'underline':
    case 'strikethrough': {
      return raw === false ? null : { type: key };
    }
    case 'font_family': {
      const v = String(raw);
      return v ? { type: 'font_family', value: v } : null;
    }
    case 'font_size': {
      const n = Number(raw);
      const value = clamp(n, 400, 12_800);
      if (value !== n) warnings.push(`font_size clamped: ${n} -> ${value}`);
      return { type: 'font_size', value };
    }
    case 'font_weight': {
      const n = Number(raw);
      const v = clamp(Math.round(n / 100) * 100, 100, 900);
      if (v !== n) warnings.push(`font_weight clamped: ${n} -> ${v}`);
      return { type: 'font_weight', value: v };
    }
    case 'text_color':
    case 'background_color': {
      const v = String(raw);
      if (!v || v === 'none') return null;
      return { type: key, value: v };
    }
    case 'letter_spacing': {
      const n = Number(raw);
      const value = clamp(n, -50, 200);
      if (value !== n) warnings.push(`letter_spacing clamped: ${n} -> ${value}`);
      return { type: 'letter_spacing', value };
    }
    case 'line_height': {
      const n = Number(raw);
      const value = clamp(n, 50, 400);
      if (value !== n) warnings.push(`line_height clamped: ${n} -> ${value}`);
      return { type: 'line_height', value };
    }
    default: {
      warnings.push(`unknown cascade key mapped to ${key}`);
      return null;
    }
  }
};

const readCascade = (entry: LegacyNodeEntry, warnings: string[]): Partial<Record<ModifierType, Modifier>> => {
  const out: Partial<Record<ModifierType, Modifier>> = {};
  for (const [rawKey, rawValue] of Object.entries(entry.cascade_attrs ?? {})) {
    const mapping = CASCADE_KEYS[rawKey];
    if (!mapping) {
      warnings.push(`dropped cascade attr: ${rawKey}`);
      continue;
    }
    const modifier = modifierFromCascade(mapping.type, rawValue, warnings);
    if (modifier) out[mapping.type] = modifier;
  }
  return out;
};

type Inherited = {
  lineHeight: number;
  styles: Partial<Record<ModifierType, Modifier>>;
  stylesAllowed: boolean;
};

const LEAF_NODE_TYPES = new Set(['image', 'file', 'embed', 'archived']);

const countLegacySubtreeChars = (entry: LegacyNodeEntry, nodes: Record<string, LegacyNodeEntry>): number => {
  let count = 0;
  const walk = (node: LegacyNodeEntry) => {
    if (node.type === 'text') {
      for (const segment of node.text ?? []) count += segment.text.replaceAll('\t', '').length;
      return;
    }
    for (const childId of node.children ?? []) {
      const child = nodes[childId];
      if (child) walk(child);
    }
  };
  for (const childId of entry.children ?? []) {
    const child = nodes[childId];
    if (child) walk(child);
  }
  return count;
};

const warnLeafSubtreeText = (nodeId: string, entry: LegacyNodeEntry, ctx: WalkCtx): void => {
  const count = countLegacySubtreeChars(entry, ctx.nodes);
  if (count > 0) {
    ctx.warnings.push(`text inside ${entry.type} dropped: ${count} chars (${nodeId})`);
  }
};

const attachRemarks = (nodeId: string, entry: LegacyNodeEntry, result: PlainNodeEntry[], ctx: WalkCtx): void => {
  const remarks = Object.entries(entry.remarks ?? {})
    .map(([remarkId, r]) => ({ id: remarkId, user_id: r.user_id, text: r.text, created_at: r.created_at }))
    .toSorted((a, b) => a.created_at - b.created_at);
  const tags = [...ctx.orphanTags];
  ctx.orphanTags.length = 0;
  if (remarks.length > 0) {
    tags.push({ nodeId, remarks });
  }
  if (tags.length === 0) return;
  const target = result[0];
  if (target) {
    pushTags(target, tags, ctx);
  } else {
    ctx.warnings.push(`remark anchor deferred to parent: converted node produced no output (${nodeId})`);
    ctx.orphanTags.push(...tags);
  }
};

const collectAnchors = (root: PlainNodeEntry, ctx: WalkCtx): RemarkAnchor[] => {
  const anchors: RemarkAnchor[] = [];
  const walk = (entry: PlainNodeEntry, path: number[]) => {
    for (const tag of ctx.anchorTags.get(entry) ?? []) {
      anchors.push({ path, nodeId: tag.nodeId, remarks: tag.remarks });
    }
    for (const [index, child] of entry.children.entries()) walk(child, [...path, index]);
  };
  walk(root, []);
  return anchors;
};

const makeEntry = (
  node: PlainNode,
  modifiers: Partial<Record<ModifierType, Modifier>>,
  children: PlainNodeEntry[],
  carry: Modifier[] = [],
): PlainNodeEntry => ({ node, modifiers: modifiers as PlainNodeEntry['modifiers'], carry, children });

const convertChildren = (entry: LegacyNodeEntry, inherited: Inherited, ctx: WalkCtx): PlainNodeEntry[] => {
  const out: PlainNodeEntry[] = [];
  for (const childId of entry.children ?? []) {
    const child = ctx.nodes[childId];
    if (!child) throw new Error(`dangling child: ${childId}`);
    out.push(...convertNode(childId, child, inherited, ctx));
  }
  return out;
};

const segmentModifiers = (segment: LegacySegment, warnings: string[]): Partial<Record<ModifierType, Modifier>> => {
  const modifiers: Partial<Record<ModifierType, Modifier>> = {};

  for (const style of segment.styles ?? []) {
    switch (style.type) {
      case 'bold':
      case 'italic':
      case 'underline':
      case 'strikethrough': {
        modifiers[style.type] = { type: style.type };
        break;
      }
      case 'font_size': {
        const value = clamp(style.size, 400, 12_800);
        if (value !== style.size) warnings.push(`font_size clamped: ${style.size} -> ${value}`);
        modifiers.font_size = { type: 'font_size', value };
        break;
      }
      case 'font_family': {
        if (style.family) modifiers.font_family = { type: 'font_family', value: style.family };
        break;
      }
      case 'font_weight': {
        const value = clamp(Math.round(style.weight / 100) * 100, 100, 900);
        if (value !== style.weight) warnings.push(`font_weight clamped: ${style.weight} -> ${value}`);
        modifiers.font_weight = { type: 'font_weight', value };
        break;
      }
      case 'text_color': {
        if (style.color && style.color !== 'none') modifiers.text_color = { type: 'text_color', value: style.color };
        break;
      }
      case 'background_color': {
        if (style.color && style.color !== 'none') modifiers.background_color = { type: 'background_color', value: style.color };
        break;
      }
      case 'letter_spacing': {
        const value = clamp(style.spacing, -50, 200);
        if (value !== style.spacing) warnings.push(`letter_spacing clamped: ${style.spacing} -> ${value}`);
        modifiers.letter_spacing = { type: 'letter_spacing', value };
        break;
      }
    }
  }

  for (const annotation of segment.annotations ?? []) {
    if (annotation.type === 'link' && annotation.href) modifiers.link = { type: 'link', href: annotation.href };
    if (annotation.type === 'ruby' && annotation.text) modifiers.ruby = { type: 'ruby', text: annotation.text };
  }

  return modifiers;
};

const convertTextNode = (entry: LegacyNodeEntry, ctx: WalkCtx, inherited: Inherited): PlainNodeEntry[] => {
  const out: PlainNodeEntry[] = [];
  for (const segment of entry.text ?? []) {
    const modifiers = inherited.stylesAllowed ? { ...inherited.styles, ...segmentModifiers(segment, ctx.warnings) } : {};
    dropInheritedEquals(modifiers, ctx);
    let tabModifiers: Partial<Record<ModifierType, Modifier>> | null = null;
    let buffer = '';
    const flush = () => {
      if (buffer) {
        out.push(makeEntry({ type: 'text', text: buffer }, modifiers, []));
        buffer = '';
      }
    };
    for (const ch of segment.text) {
      if (ch === '\t') {
        flush();
        if (tabModifiers === null) {
          tabModifiers = { ...modifiers };
          if (tabModifiers.link || tabModifiers.ruby) {
            delete tabModifiers.link;
            delete tabModifiers.ruby;
            ctx.warnings.push('link/ruby dropped from tab: v2 schema does not allow them on tab nodes');
          }
        }
        out.push(makeEntry({ type: 'tab' }, tabModifiers, []));
      } else {
        buffer += ch;
      }
    }
    flush();
  }
  return out;
};

const convertNode = (nodeId: string, entry: LegacyNodeEntry, inherited: Inherited, ctx: WalkCtx): PlainNodeEntry[] => {
  const result = convertNodeInner(nodeId, entry, inherited, ctx);
  attachRemarks(nodeId, entry, result, ctx);
  return result;
};

const convertNodeInner = (nodeId: string, entry: LegacyNodeEntry, inherited: Inherited, ctx: WalkCtx): PlainNodeEntry[] => {
  const cascade = readCascade(entry, ctx.warnings);
  const cascadeLineHeight = cascade.line_height?.type === 'line_height' ? cascade.line_height.value : null;
  const cascadeStyles: Partial<Record<ModifierType, Modifier>> = { ...cascade };
  delete cascadeStyles.line_height;
  const childInherited: Inherited = {
    lineHeight: cascadeLineHeight ?? inherited.lineHeight,
    styles: { ...inherited.styles, ...cascadeStyles },
    stylesAllowed: inherited.stylesAllowed,
  };

  switch (entry.type) {
    case 'paragraph': {
      const modifiers: Partial<Record<ModifierType, Modifier>> = {};
      const align = String(entry.align ?? 'left');
      if (align !== 'left') {
        modifiers.alignment = { type: 'alignment', value: align as 'left' | 'center' | 'right' | 'justify' };
      }
      const isEmpty = (entry.children ?? []).length === 0;
      if (isEmpty && cascadeLineHeight != null) {
        if (cascadeLineHeight !== inherited.lineHeight) {
          modifiers.line_height = { type: 'line_height', value: cascadeLineHeight };
        }
      } else {
        const lineHeight = clamp(Number(entry.line_height ?? childInherited.lineHeight), 50, 400);
        if (lineHeight !== childInherited.lineHeight) {
          modifiers.line_height = { type: 'line_height', value: lineHeight };
        }
      }
      const carrySource = { ...childInherited.styles };
      dropInheritedEquals(carrySource, ctx);
      const carry = isEmpty && inherited.stylesAllowed ? sortCarry(Object.values(carrySource)) : [];
      const inlines = mergeAdjacentTextRuns(convertChildren(entry, childInherited, ctx), ctx);
      const chunks: PlainNodeEntry[][] = [[]];
      for (const inline of inlines) {
        chunks.at(-1)?.push(inline);
        if (inline.node.type === 'page_break') chunks.push([]);
      }
      if (chunks.length > 1 && chunks.at(-1)?.length === 0) chunks.pop();
      if (chunks.length > 1) {
        ctx.warnings.push(`paragraph split at page_break into ${chunks.length} paragraphs`);
      }
      return chunks.map((chunk, index) => makeEntry({ type: 'paragraph' }, { ...modifiers }, chunk, index === 0 ? carry : []));
    }
    case 'blockquote': {
      return [
        makeEntry(
          { type: 'blockquote', variant: entry.variant as never },
          {},
          normalizeBlockChildren(convertChildren(entry, childInherited, ctx), ctx),
        ),
      ];
    }
    case 'callout': {
      return [
        makeEntry(
          { type: 'callout', variant: entry.variant as never },
          {},
          normalizeBlockChildren(convertChildren(entry, childInherited, ctx), ctx),
        ),
      ];
    }
    case 'fold_title': {
      const inner: Inherited = { lineHeight: childInherited.lineHeight, styles: {}, stylesAllowed: false };
      const children = mergeAdjacentTextRuns(convertChildren(entry, inner, ctx), ctx);
      const texts = children.filter((child) => child.node.type === 'text');
      if (texts.length !== children.length) {
        ctx.warnings.push(
          `fold_title children other than text dropped: ${children
            .filter((child) => child.node.type !== 'text')
            .map((child) => child.node.type)
            .join(', ')}`,
        );
        for (const dropped of children) {
          if (dropped.node.type === 'text') continue;
          const tags = ctx.anchorTags.get(dropped);
          if (tags) {
            ctx.anchorTags.delete(dropped);
            ctx.orphanTags.push(...tags);
          }
        }
      }
      return [makeEntry({ type: 'fold_title' }, {}, mergeAdjacentTextRuns(texts, ctx), [])];
    }
    case 'list_item': {
      const children = normalizeBlockChildren(convertChildren(entry, childInherited, ctx), ctx);
      return rebuildListItems(children, ctx);
    }
    case 'fold_content': {
      return [makeEntry({ type: 'fold_content' }, {}, normalizeBlockChildren(convertChildren(entry, childInherited, ctx), ctx))];
    }
    case 'bullet_list':
    case 'ordered_list': {
      return [makeEntry({ type: entry.type }, {}, convertChildren(entry, childInherited, ctx))];
    }
    case 'fold': {
      return [makeEntry({ type: 'fold' }, {}, rebuildFoldChildren(convertChildren(entry, childInherited, ctx), ctx))];
    }
    case 'table_row': {
      return [makeEntry({ type: 'table_row' }, {}, rebuildRowCells(convertChildren(entry, childInherited, ctx), ctx))];
    }
    case 'table': {
      const modifiers: Partial<Record<ModifierType, Modifier>> = {};
      const tableAlign = String(entry.align ?? 'left');
      if (tableAlign !== 'left') {
        modifiers.alignment = { type: 'alignment', value: tableAlign as 'left' | 'center' | 'right' };
      }
      const rows = rebuildTableRows(convertChildren(entry, childInherited, ctx), ctx);
      const maxCols = Math.max(0, ...rows.map((row) => row.children.length));
      for (const row of rows) {
        while (row.children.length < maxCols) {
          row.children.push(makeEntry({ type: 'table_cell', col_width: undefined, background_color: undefined }, {}, [emptyParagraph()]));
        }
      }
      return [
        makeEntry(
          { type: 'table', border_style: entry.border_style as never, proportion: toProportion(entry.proportion) },
          modifiers,
          rows,
        ),
      ];
    }
    case 'table_cell': {
      const modifiers: Partial<Record<ModifierType, Modifier>> = {};
      const background = childInherited.styles.background_color;
      if (background) {
        modifiers.background_color = background;
      }
      const cellInherited: Inherited = { ...childInherited, styles: { ...childInherited.styles } };
      delete cellInherited.styles.background_color;
      const cellChildren = normalizeBlockChildren(convertChildren(entry, cellInherited, ctx), ctx);
      if (cellChildren.length === 0) {
        cellChildren.push(emptyParagraph());
      }
      return [
        makeEntry({ type: 'table_cell', col_width: toColWidth(entry.col_width), background_color: undefined }, modifiers, cellChildren),
      ];
    }
    case 'image': {
      warnLeafSubtreeText(nodeId, entry, ctx);
      return [
        makeEntry({ type: 'image', id: (entry.id as string | undefined) ?? undefined, proportion: toProportion(entry.proportion) }, {}, []),
      ];
    }
    case 'file': {
      warnLeafSubtreeText(nodeId, entry, ctx);
      return [makeEntry({ type: 'file', id: (entry.id as string | undefined) ?? undefined }, {}, [])];
    }
    case 'embed': {
      warnLeafSubtreeText(nodeId, entry, ctx);
      return [makeEntry({ type: 'embed', id: (entry.id as string | undefined) ?? undefined }, {}, [])];
    }
    case 'archived': {
      warnLeafSubtreeText(nodeId, entry, ctx);
      return [makeEntry({ type: 'archived', id: (entry.id as string | undefined) ?? undefined }, {}, [])];
    }
    case 'hard_break': {
      return [makeEntry({ type: 'hard_break' }, {}, [])];
    }
    case 'horizontal_rule': {
      return [makeEntry({ type: 'horizontal_rule', variant: entry.variant as never }, {}, [])];
    }
    case 'page_break': {
      return [makeEntry({ type: 'page_break' }, {}, [])];
    }
    case 'text': {
      return convertTextNode(entry, ctx, childInherited);
    }
    default: {
      throw new Error(`unknown legacy node type: ${entry.type} (${nodeId})`);
    }
  }
};

const LIST_ITEM_CHILD_TYPES = new Set(['paragraph', 'bullet_list', 'ordered_list']);

const rebuildListItems = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  if (children.some((child) => !LIST_ITEM_CHILD_TYPES.has(child.node.type))) {
    ctx.warnings.push(`list_item child not representable in v2 schema: ${children.map((c) => c.node.type).join(', ')}`);
    return [makeEntry({ type: 'list_item' }, {}, children)];
  }

  const segments: { head: PlainNodeEntry; list: PlainNodeEntry | null }[] = [];
  let current: (typeof segments)[number] | null = null;
  for (const child of children) {
    if (child.node.type === 'paragraph') {
      if (current && current.list === null) {
        if (modifiersSignature(child.modifiers) !== modifiersSignature(current.head.modifiers)) {
          ctx.warnings.push(`paragraph modifiers dropped in list_item merge: ${JSON.stringify(child.modifiers)}`);
        }
        current.head.children = mergeAdjacentTextRuns(
          [...current.head.children, makeEntry({ type: 'hard_break' }, {}, []), ...child.children],
          ctx,
        );
        current.head.carry = [];
        transferTags(child, current.head, ctx);
        ctx.warnings.push('list_item paragraphs merged with hard_break');
      } else {
        current = { head: child, list: null };
        segments.push(current);
      }
    } else {
      if (current && current.list === null) {
        current.list = child;
      } else {
        current = { head: emptyParagraph(), list: child };
        segments.push(current);
      }
    }
  }
  if (segments.length === 0) {
    segments.push({ head: emptyParagraph(), list: null });
  }
  if (segments.length > 1) {
    ctx.warnings.push(`list_item split into ${segments.length} items`);
  }
  return segments.map((segment) => makeEntry({ type: 'list_item' }, {}, segment.list ? [segment.head, segment.list] : [segment.head]));
};

const makeCell = (children: PlainNodeEntry[]): PlainNodeEntry =>
  makeEntry({ type: 'table_cell', col_width: undefined, background_color: undefined }, {}, children);

const rebuildRowCells = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  const cells: PlainNodeEntry[] = [];
  let inlineBuffer: PlainNodeEntry[] = [];
  const flushInlines = () => {
    if (inlineBuffer.length > 0) {
      const wrapped = emptyParagraph();
      wrapped.children = mergeAdjacentTextRuns(inlineBuffer, ctx);
      cells.push(makeCell([wrapped]));
      inlineBuffer = [];
      ctx.warnings.push('table_row inline children wrapped into table_cell');
    }
  };
  for (const child of children) {
    if (INLINE_NODE_TYPES.has(child.node.type)) {
      inlineBuffer.push(child);
      continue;
    }
    flushInlines();
    if (child.node.type === 'table_cell') {
      cells.push(child);
    } else {
      cells.push(makeCell([child]));
      ctx.warnings.push(`table_row child wrapped into table_cell: ${child.node.type}`);
    }
  }
  flushInlines();
  return cells;
};

const rebuildTableRows = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  const rows: PlainNodeEntry[] = [];
  let inlineBuffer: PlainNodeEntry[] = [];
  const flushInlines = () => {
    if (inlineBuffer.length > 0) {
      const wrapped = emptyParagraph();
      wrapped.children = mergeAdjacentTextRuns(inlineBuffer, ctx);
      rows.push(makeEntry({ type: 'table_row' }, {}, [makeCell([wrapped])]));
      inlineBuffer = [];
      ctx.warnings.push('table inline children wrapped into table_row');
    }
  };
  for (const child of children) {
    if (INLINE_NODE_TYPES.has(child.node.type)) {
      inlineBuffer.push(child);
      continue;
    }
    flushInlines();
    if (child.node.type === 'table_row') {
      rows.push(child);
    } else {
      rows.push(makeEntry({ type: 'table_row' }, {}, [makeCell([child])]));
      ctx.warnings.push(`table child wrapped into table_row: ${child.node.type}`);
    }
  }
  flushInlines();
  return rows;
};

const rebuildFoldChildren = (children: PlainNodeEntry[], ctx: WalkCtx): PlainNodeEntry[] => {
  if (children.some((child) => child.node.type !== 'fold_title' && child.node.type !== 'fold_content')) {
    ctx.warnings.push(`fold child not representable in v2 schema: ${children.map((c) => c.node.type).join(', ')}`);
    return children;
  }

  const titles = children.filter((child) => child.node.type === 'fold_title');
  const contents = children.filter((child) => child.node.type === 'fold_content');

  const title = titles[0] ?? makeEntry({ type: 'fold_title' }, {}, []);
  for (const extra of titles.slice(1)) {
    title.children = mergeAdjacentTextRuns([...title.children, ...extra.children], ctx);
    transferTags(extra, title, ctx);
  }
  if (titles.length !== 1) {
    ctx.warnings.push(`fold normalized to a single fold_title (had ${titles.length})`);
  }

  const content = contents[0] ?? makeEntry({ type: 'fold_content' }, {}, []);
  for (const extra of contents.slice(1)) {
    content.children.push(...extra.children);
    transferTags(extra, content, ctx);
  }
  content.children = normalizeBlockChildren(content.children, ctx);
  if (content.children.length === 0) {
    content.children.push(emptyParagraph());
  }
  if (contents.length !== 1) {
    ctx.warnings.push(`fold normalized to a single fold_content (had ${contents.length})`);
  }

  return [title, content];
};

export const convertLegacyDocumentJson = (json: LegacyDocumentJson): ConvertResult => {
  const ctx: WalkCtx = { nodes: json.nodes, anchorTags: new WeakMap(), orphanTags: [], warnings: [], rootEffective: {} };

  const root = json.nodes[ROOT_ID];
  if (!root || root.type !== 'root') throw new Error('missing root node');

  const modifiers = readCascade(root, ctx.warnings);
  for (const ty of INHERITABLE_RUN_KINDS) {
    ctx.rootEffective[ty] = modifiers[ty] ?? INHERITABLE_TEXT_DEFAULTS[ty];
  }
  if (modifiers.text_color) {
    if (!isDefaultTextColor(modifiers.text_color)) {
      ctx.warnings.push(`document default text_color dropped: v2 has no document-level color (${JSON.stringify(modifiers.text_color)})`);
    }
    delete modifiers.text_color;
  }
  if (modifiers.background_color) {
    ctx.warnings.push(
      `document default background_color dropped: v2 has no document-level color (${JSON.stringify(modifiers.background_color)})`,
    );
    delete modifiers.background_color;
  }
  if (json.settings.block_gap != null) {
    modifiers.block_gap = { type: 'block_gap', value: clamp(json.settings.block_gap, 0, 400) };
  }
  if (json.settings.paragraph_indent != null) {
    modifiers.paragraph_indent = { type: 'paragraph_indent', value: clamp(json.settings.paragraph_indent, 0, 400) };
  }

  const inheritedLineHeight = modifiers.line_height?.type === 'line_height' ? modifiers.line_height.value : 160;
  const layoutMode = roundLayoutMode(json.settings.layout_mode ?? { type: 'continuous', max_width: 600 });

  const children = normalizeBlockChildren(
    convertChildren(root, { lineHeight: inheritedLineHeight, styles: {}, stylesAllowed: true }, ctx),
    ctx,
  );
  if (children.at(-1)?.node.type !== 'paragraph') {
    children.push(emptyParagraph());
  }

  const rootEntry = makeEntry({ type: 'root', layout_mode: layoutMode }, modifiers, children);
  attachRemarks(ROOT_ID, root, [rootEntry], ctx);

  const plain: PlainDoc = { root: rootEntry };

  return { plain, remarkAnchors: collectAnchors(rootEntry, ctx), warnings: ctx.warnings };
};

const TEXT_CONTAINERS = new Set([
  'root',
  'paragraph',
  'blockquote',
  'callout',
  'bullet_list',
  'ordered_list',
  'list_item',
  'fold',
  'fold_title',
  'fold_content',
  'table',
  'table_row',
  'table_cell',
]);

export const deriveExpectedTextFromPlain = (plain: PlainDoc): string => {
  let out = '';
  const walk = (entry: PlainNodeEntry) => {
    if (entry.node.type === 'text') {
      out += entry.node.text;
      return;
    }
    if (!TEXT_CONTAINERS.has(entry.node.type)) return;
    for (const child of entry.children) walk(child);
    out += '\n';
  };
  walk(plain.root);
  return out.replace(/\n+$/, '');
};

export const collectLegacyTextChars = (json: LegacyDocumentJson): string => {
  let out = '';
  const walk = (nodeId: string) => {
    const entry = json.nodes[nodeId];
    if (!entry) return;
    if (entry.type === 'text') {
      for (const segment of entry.text ?? []) out += segment.text.replaceAll('\t', '');
      return;
    }
    if (LEAF_NODE_TYPES.has(entry.type)) return;
    for (const childId of entry.children ?? []) walk(childId);
  };
  walk(ROOT_ID);
  return out;
};

export const collectPlainTextChars = (plain: PlainDoc): string => {
  let out = '';
  const walk = (entry: PlainNodeEntry) => {
    if (entry.node.type === 'text') out += entry.node.text;
    for (const child of entry.children) walk(child);
  };
  walk(plain.root);
  return out;
};

export const canonical = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .filter(([, v]) => v !== undefined)
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([k, v]) => [k, canonical(v)]),
    );
  }
  return value;
};

export const plainStructureEquals = (a: PlainDoc, b: PlainDoc): boolean => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));

const DIFF_LIMIT = 5;

const truncate = (value: unknown): string => {
  const s = JSON.stringify(value) ?? 'undefined';
  return s.length > 80 ? `${s.slice(0, 77)}...` : s;
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const summarizeEntry = (value: unknown): string => {
  if (!isPlainObject(value) || !isPlainObject(value.node)) return truncate(value);
  const node = value.node as { type?: string; text?: string };
  const text = typeof node.text === 'string' ? `"${node.text.slice(0, 12)}"` : '';
  const keys = isPlainObject(value.modifiers) ? Object.keys(value.modifiers).join(',') : '';
  return `${node.type ?? '?'}${text ? `(${text})` : ''}${keys ? `{${keys}}` : ''}`;
};

const collectDiffs = (a: unknown, b: unknown, path: string, out: string[]): void => {
  if (out.length >= DIFF_LIMIT) return;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      out.push(
        `${path}: array length ${a.length} != ${b.length} — a=[${a.map(summarizeEntry).join(' ')}] b=[${b.map(summarizeEntry).join(' ')}]`,
      );
      return;
    }
    for (const [i, item] of a.entries()) collectDiffs(item, b[i], `${path}[${i}]`, out);
    return;
  }
  if (isPlainObject(a) && isPlainObject(b)) {
    for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
      if (out.length >= DIFF_LIMIT) return;
      const av = a[key];
      const bv = b[key];
      if (av === undefined || bv === undefined) {
        out.push(`${path}.${key}: ${truncate(av)} != ${truncate(bv)}`);
      } else {
        collectDiffs(av, bv, `${path}.${key}`, out);
      }
    }
    return;
  }
  if (JSON.stringify(a) !== JSON.stringify(b)) {
    out.push(`${path}: ${truncate(a)} != ${truncate(b)}`);
  }
};

export const plainStructureDiff = (a: PlainDoc, b: PlainDoc): string[] => {
  const out: string[] = [];
  collectDiffs(canonical(a), canonical(b), 'doc', out);
  return out;
};

export const firstTextDiff = (a: string, b: string): string => {
  const max = Math.min(a.length, b.length);
  let i = 0;
  while (i < max && a[i] === b[i]) i += 1;
  const from = Math.max(0, i - 15);
  return `at ${i}: ${JSON.stringify(a.slice(from, i + 25))} != ${JSON.stringify(b.slice(from, i + 25))} (len ${a.length}/${b.length})`;
};
