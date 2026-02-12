import type { ExternalElementData, Rect, TextAlign } from '@typie/editor';

export type {
  Affinity,
  Annotation,
  AnnotationType,
  BlockquoteVariant,
  Direction,
  ExternalElementData,
  HorizontalRuleVariant,
  LayoutMode,
  Message,
  Node,
  NodeType,
  PointerStyle,
  Position,
  Rect,
  Style,
  StyleType,
  TextAlign,
  TextBound,
  Theme,
} from '@typie/editor';

export type ExternalElement = {
  pageIdx: number;
  nodeId: string;
  bounds: Rect;
  data: ExternalElementData;
  isSelected: boolean;
};

export type SelectionStats = {
  uniformAlign: TextAlign | undefined;
  uniformLineHeight: number | undefined;
};

export type AiFeedbackData = {
  id: string;
  nodeId: string;
  startOffset: number;
  endOffset: number;
  startText: string;
  endText: string;
  feedback: string;
};

export type SpellcheckErrorData = {
  id: string;
  nodeId: string;
  startOffset: number;
  endOffset: number;
  context: string;
  corrections: string[];
  explanation: string;
};

export type ImageAsset = {
  id: string;
  url: string;
  width: number;
  height: number;
  placeholder: string;
};

export type PasteData = { type: 'pasteHtml'; html: string; text: string } | { type: 'pasteText'; text: string };

export type FileAsset = {
  id: string;
  url: string;
  name: string;
  size: number;
};

export type EmbedAsset = {
  id: string;
  url: string;
  title: string | null;
  description: string | null;
  thumbnailUrl: string | null;
  html: string | null;
};

export type ArchivedAsset = {
  id: string;
  content: string;
};
