import type { Alignment, PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/server';
import type { EmbedInfo, ImageAsset, PageLayout } from '../types.ts';

export type RunStyle = {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strikethrough: boolean;
  fontFamily: string;
  fontSizePt100: number;
  fontWeight: number;
  textColorHex?: string;
  backgroundColorHex?: string;
  letterSpacing: number;
  link?: string;
  ruby?: string;
};
export type Run = { text: string; style: RunStyle };
export type Inline = { type: 'run'; run: Run } | { type: 'hard_break' } | { type: 'page_break' } | { type: 'tab' };
export type ParagraphV2 = { inlines: Inline[]; align: Alignment; lineHeight: number };
export type ImageV2 = { id: string; proportion: number; asset: ImageAsset };
export type FileV2 = { id: string };
export type EmbedV2 = { id: string; data: EmbedInfo | undefined };
export type ArchivedV2 = { id: string };
export type TableCellV2<T> = { children: T[]; colWidth?: number; backgroundColorHex?: string };
export type TableRowV2<T> = { cells: TableCellV2<T>[] };
export type TableV2<T> = { rows: TableRowV2<T>[]; borderStyle: 'solid' | 'dashed' | 'dotted' | 'none'; proportion: number };
export type DocDefaultsV2 = {
  fontFamily: string;
  fontSizePt100: number;
  lineHeight: number;
  paragraphIndentPx: number;
  blockGapPx: number;
};
export type ParsedDocumentV2 = {
  plain: PlainDoc;
  root: PlainNodeEntry;
  defaults: DocDefaultsV2;
  layout: PageLayout | undefined;
  images: Map<string, ImageAsset>;
  embeds: Map<string, EmbedInfo>;
};
export type NodeVisitorV2<TCtx, TOut> = {
  paragraph: (p: ParagraphV2, ctx: TCtx) => TOut;
  table: (t: TableV2<TOut>, ctx: TCtx) => TOut;
  image: (n: ImageV2, ctx: TCtx) => TOut;
  file: (n: FileV2, ctx: TCtx) => TOut;
  embed: (n: EmbedV2, ctx: TCtx) => TOut;
  archived: (n: ArchivedV2, ctx: TCtx) => TOut;
  horizontalRule: (variant: string, ctx: TCtx) => TOut;
  bulletList: (items: TOut[][], ctx: TCtx) => TOut;
  orderedList: (items: TOut[][], ctx: TCtx) => TOut;
  blockquote: (variant: string, children: TOut[], ctx: TCtx) => TOut;
  callout: (variant: string, children: TOut[], ctx: TCtx) => TOut;
  fold: (title: Run[], content: TOut[], ctx: TCtx) => TOut;
  onEnterList?: (type: 'bullet' | 'ordered', depth: number, ctx: TCtx) => void;
  onExitList?: (ctx: TCtx) => void;
};

export { type EmbedInfo, type ExportFontFamily, type ImageAsset, type PageLayout } from '../types.ts';
