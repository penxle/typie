import type { ExternalElementData, Position, Rect, TextAlign } from '@typie/editor';

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

export type LinkAnnotationValue = { href: string; [key: string]: string };
export type RubyAnnotationValue = { text: string; [key: string]: string };

export type SelectionEndpointBounds = {
  pageIdx: number;
  bounds: Rect;
};

export type Selection = {
  collapsed: boolean;
  cmp: number;
  anchor: Position;
  head: Position;
  anchorBounds: SelectionEndpointBounds | null;
  headBounds: SelectionEndpointBounds | null;
};

export type Attribute =
  | { type: 'text_align'; values: (TextAlign | null)[] }
  | { type: 'line_height'; values: (number | null)[] }
  | { type: 'background_color'; values: (string | null)[] }
  | { type: 'text_color'; values: (string | null)[] }
  | { type: 'font_size'; values: (number | null)[] }
  | { type: 'font_family'; values: (string | null)[] }
  | { type: 'font_weight'; values: (number | null)[] }
  | { type: 'letter_spacing'; values: (number | null)[] }
  | { type: 'italic'; values: (true | null)[] }
  | { type: 'strikethrough'; values: (true | null)[] }
  | { type: 'underline'; values: (true | null)[] }
  | { type: 'link'; values: (LinkAnnotationValue | null)[] }
  | { type: 'ruby'; values: (RubyAnnotationValue | null)[] };

export type AiFeedback = {
  id: string;
  startText: string;
  endText: string;
  feedback: string;
  active: boolean;
};

export type SpellcheckError = {
  id: string;
  context: string;
  corrections: string[];
  explanation: string;
  active: boolean;
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
