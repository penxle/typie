import type { EmbedInfo, ExportFontFamily, ImageAsset, NodeEntry, PageLayout } from '../core/types.ts';
import type { DocInfoTables } from './doc-info.ts';

export type HwpConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, EmbedInfo>;
  tables: DocInfoTables;
  pageLayout: PageLayout;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  fonts: ExportFontFamily[];
  sectionDefEmitted: boolean;
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
