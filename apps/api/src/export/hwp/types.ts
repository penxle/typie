import type { ImageAsset } from '../external';
import type { FontNameMap } from '../font';
import type { DocInfoTables } from './doc-info';

export type Style =
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

export type Annotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

export type TextSegment = {
  text: string;
  styles: Style[];
  annotations: Annotation[];
};

export type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

export type PageLayout = {
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
};

export type HwpConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, { url: string; title: string | null }>;
  tables: DocInfoTables;
  pageLayout: PageLayout;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  fontNameMap: FontNameMap;
  defaultFamilyName: string;
  defaultFontId: number;
  defaultCharShapeId: number;
  defaultParaShapeId: number;
  paragraphIndentHwp: number;
  blockGapHwp: number;
  defaultFontSizePt100: number;
  defaultLineHeight: number;
  instanceCounter: number;
};

export type InlineSegment = {
  text: string;
  charShapeId: number;
  link?: string;
  ruby?: string;
  rubyCharShapeId?: number;
};
