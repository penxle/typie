export type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

export type DocumentJson = {
  settings: Record<string, unknown>;
  nodes: Record<string, NodeEntry>;
};

// -- 인라인 --

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
  type: 'text';
  text: string;
  styles: Style[];
  annotations: Annotation[];
};

export type HardBreak = { type: 'hard_break' };
export type PageBreak = { type: 'page_break' };
export type InlineSegment = TextSegment | HardBreak | PageBreak;

// -- 블록 데이터 --

export type ParagraphData = {
  segments: InlineSegment[];
  attrs: Record<string, unknown>;
};

export type TableCellData<T> = {
  children: T[];
  attrs: Record<string, unknown>;
};

export type TableData<T> = {
  rows: { cells: TableCellData<T>[] }[];
};

export type ImageData = {
  id: string;
  attrs: Record<string, unknown>;
};

export type FileData = {
  id: string;
  attrs: Record<string, unknown>;
};

export type EmbedData = {
  url: string;
  title: string | null;
};

export type CalloutData = {
  variant: string;
};

export type ArchivedData = {
  attrs: Record<string, unknown>;
};

// -- 폰트 --

export type ExportFont = {
  weight: number;
  url: string;
  name: string;
  localizedName?: string;
  postScriptName: string;
};

export type ExportFontFamily = {
  family: string;
  weights: ExportFont[];
};

// -- 레이아웃 --

export type PageLayout = {
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
};

// -- 진입점 --

export type ExportFormat = 'hwp' | 'docx' | 'epub' | 'pdf';

export type ExportOptions = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  fonts: ExportFontFamily[];
  layout?: PageLayout;
};

// -- 에셋 --

export type ImageAsset = {
  type: 'image';
  id: string;
  format: string;
  width: number;
  height: number;
  bytes: Uint8Array;
};

export type EmbedInfo = {
  url: string;
  title: string | null;
};
