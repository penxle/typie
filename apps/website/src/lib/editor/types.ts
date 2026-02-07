export type {
  Affinity,
  AiFeedbackOverlay,
  Cmd,
  Direction,
  ExternalElement,
  ExternalElementData,
  HorizontalRuleVariant,
  LayoutMode,
  Mark,
  MarkType,
  Message,
  Node,
  NodeType,
  PointerStyle,
  Position,
  Rect,
  SearchOverlay,
  SelectionStats,
  SpellcheckOverlay,
  TextAlign,
  TextBound,
  Theme,
} from '@typie/editor';

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
