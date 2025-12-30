export type {
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
  Rect,
  SelectionStats,
  SpellcheckOverlay,
  TextAlign,
  TextBound,
  Theme,
  WritingSystem,
} from '@typie/editor';

export type SpellcheckErrorData = {
  id: string;
  nodeId: string;
  startOffset: number;
  endOffset: number;
  context: string;
  corrections: string[];
  explanation: string;
};
