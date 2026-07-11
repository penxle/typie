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

const mergeAdjacentTextRuns = (children: PlainNodeEntry[]): PlainNodeEntry[] => {
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
    } else {
      out.push(child);
    }
  }
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

type WalkCtx = {
  nodes: Record<string, LegacyNodeEntry>;
  remarkAnchors: RemarkAnchor[];
  warnings: string[];
  rootEffective: Partial<Record<ModifierType, Modifier>>;
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

const collectRemarks = (nodeId: string, entry: LegacyNodeEntry, path: number[], ctx: WalkCtx) => {
  const remarks = Object.entries(entry.remarks ?? {})
    .map(([remarkId, r]) => ({ id: remarkId, user_id: r.user_id, text: r.text, created_at: r.created_at }))
    .toSorted((a, b) => a.created_at - b.created_at);
  if (remarks.length > 0) {
    ctx.remarkAnchors.push({ path, nodeId, remarks });
  }
};

const makeEntry = (
  node: PlainNode,
  modifiers: Partial<Record<ModifierType, Modifier>>,
  children: PlainNodeEntry[],
  carry: Modifier[] = [],
): PlainNodeEntry => ({ node, modifiers: modifiers as PlainNodeEntry['modifiers'], carry, children });

const convertChildren = (entry: LegacyNodeEntry, path: number[], inherited: Inherited, ctx: WalkCtx): PlainNodeEntry[] => {
  const out: PlainNodeEntry[] = [];
  for (const childId of entry.children ?? []) {
    const child = ctx.nodes[childId];
    if (!child) throw new Error(`dangling child: ${childId}`);
    out.push(...convertNode(childId, child, [...path, out.length], inherited, ctx));
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

const convertNode = (nodeId: string, entry: LegacyNodeEntry, path: number[], inherited: Inherited, ctx: WalkCtx): PlainNodeEntry[] => {
  collectRemarks(nodeId, entry, path, ctx);

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
      return [makeEntry({ type: 'paragraph' }, modifiers, mergeAdjacentTextRuns(convertChildren(entry, path, childInherited, ctx)), carry)];
    }
    case 'blockquote': {
      return [makeEntry({ type: 'blockquote', variant: entry.variant as never }, {}, convertChildren(entry, path, childInherited, ctx))];
    }
    case 'callout': {
      return [makeEntry({ type: 'callout', variant: entry.variant as never }, {}, convertChildren(entry, path, childInherited, ctx))];
    }
    case 'fold_title': {
      const inner: Inherited = { lineHeight: childInherited.lineHeight, styles: {}, stylesAllowed: false };
      return [makeEntry({ type: 'fold_title' }, {}, mergeAdjacentTextRuns(convertChildren(entry, path, inner, ctx)), [])];
    }
    case 'bullet_list':
    case 'ordered_list':
    case 'list_item':
    case 'fold':
    case 'fold_content':
    case 'table_row': {
      return [makeEntry({ type: entry.type }, {}, convertChildren(entry, path, childInherited, ctx))];
    }
    case 'table': {
      const modifiers: Partial<Record<ModifierType, Modifier>> = {};
      const tableAlign = String(entry.align ?? 'left');
      if (tableAlign !== 'left') {
        modifiers.alignment = { type: 'alignment', value: tableAlign as 'left' | 'center' | 'right' };
      }
      return [
        makeEntry(
          { type: 'table', border_style: entry.border_style as never, proportion: toProportion(entry.proportion) },
          modifiers,
          convertChildren(entry, path, childInherited, ctx),
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
      return [
        makeEntry(
          { type: 'table_cell', col_width: toColWidth(entry.col_width), background_color: undefined },
          modifiers,
          convertChildren(entry, path, cellInherited, ctx),
        ),
      ];
    }
    case 'image': {
      return [
        makeEntry({ type: 'image', id: (entry.id as string | undefined) ?? undefined, proportion: toProportion(entry.proportion) }, {}, []),
      ];
    }
    case 'file': {
      return [makeEntry({ type: 'file', id: (entry.id as string | undefined) ?? undefined }, {}, [])];
    }
    case 'embed': {
      return [makeEntry({ type: 'embed', id: (entry.id as string | undefined) ?? undefined }, {}, [])];
    }
    case 'archived': {
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

export const convertLegacyDocumentJson = (json: LegacyDocumentJson): ConvertResult => {
  const ctx: WalkCtx = { nodes: json.nodes, remarkAnchors: [], warnings: [], rootEffective: {} };

  const root = json.nodes[ROOT_ID];
  if (!root || root.type !== 'root') throw new Error('missing root node');

  collectRemarks(ROOT_ID, root, [], ctx);

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

  const children = convertChildren(root, [], { lineHeight: inheritedLineHeight, styles: {}, stylesAllowed: true }, ctx);

  const plain: PlainDoc = {
    root: makeEntry({ type: 'root', layout_mode: layoutMode }, modifiers, children),
  };

  return { plain, remarkAnchors: ctx.remarkAnchors, warnings: ctx.warnings };
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

const collectDiffs = (a: unknown, b: unknown, path: string, out: string[]): void => {
  if (out.length >= DIFF_LIMIT) return;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      out.push(`${path}: array length ${a.length} != ${b.length}`);
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
