/* tslint:disable */
/* eslint-disable */
export function getMemory(): any;
export type Mark = ({ type: "background_color" } & BackgroundColorMark) | ({ type: "text_color" } & TextColorMark) | ({ type: "font_size" } & FontSizeMark) | ({ type: "font_family" } & FontFamilyMark) | ({ type: "font_weight" } & FontWeightMark) | ({ type: "italic" } & ItalicMark) | ({ type: "letter_spacing" } & LetterSpacingMark) | ({ type: "ruby" } & RubyMark) | ({ type: "strikethrough" } & StrikethroughMark) | ({ type: "underline" } & UnderlineMark);

export type MarkType = "background_color" | "text_color" | "font_size" | "font_family" | "font_weight" | "italic" | "letter_spacing" | "ruby" | "strikethrough" | "underline";

export type NodeType = "root" | "paragraph" | "blockquote" | "callout" | "text" | "image" | "hard_break" | "horizontal_rule" | "page_break" | "bullet_list" | "ordered_list" | "list_item" | "fold" | "fold_title" | "fold_content";

export type Node = ({ type: "root" } & RootNode) | ({ type: "paragraph" } & ParagraphNode) | ({ type: "blockquote" } & BlockquoteNode) | ({ type: "callout" } & CalloutNode) | ({ type: "image" } & ImageNode) | ({ type: "text" } & TextNode) | ({ type: "hard_break" } & HardBreakNode) | ({ type: "horizontal_rule" } & HorizontalRuleNode) | ({ type: "page_break" } & PageBreakNode) | ({ type: "bullet_list" } & BulletListNode) | ({ type: "ordered_list" } & OrderedListNode) | ({ type: "list_item" } & ListItemNode) | ({ type: "fold" } & FoldNode) | ({ type: "fold_title" } & FoldTitleNode) | ({ type: "fold_content" } & FoldContentNode);

export interface TextColorMark {
    key: string;
}

export interface FontFamilyMark {
    family: string;
}

export interface FontWeightMark {
    weight: number;
}

export type StrikethroughMark = undefined;

export interface LetterSpacingMark {
    spacing: number;
}

export interface BackgroundColorMark {
    key: string;
}

export interface RubyMark {
    text: string;
}

export type ItalicMark = undefined;

export interface FontSizeMark {
    size: number;
}

export type UnderlineMark = undefined;

export interface BlockquoteNode {}

export interface FoldTitleNode {}

export interface HardBreakNode {}

export interface PageBreakNode {}

export interface BulletListNode {}

export interface FoldContentNode {}

export interface OrderedListNode {}

export type HorizontalRuleVariant = "line" | "dashed_line" | "circle_line" | "diamond_line" | "circle" | "diamond" | "three_circles" | "three_diamonds" | "zigzag";

export interface HorizontalRuleNode {
    variant?: HorizontalRuleVariant;
}

export interface FoldNode {}

export interface RootNode {}

export interface TextNode {
    text: Text;
}

export interface ImageNode {
    src: string;
    width: number;
    height: number;
}

export interface CalloutNode {
    callout_type?: CalloutType;
}

export type CalloutType = "info" | "success" | "warning" | "danger";

export interface ListItemNode {}

export interface ParagraphNode {
    align?: TextAlign;
    line_height?: number;
}

export type TextAlign = "left" | "center" | "right" | "justify";

export type LayoutMode = { type: "paginated"; pageWidth: number; pageHeight: number; pageMarginTop: number; pageMarginBottom: number; pageMarginLeft: number; pageMarginRight: number } | { type: "continuous"; maxWidth: number };

export type WritingSystem = "latin" | "korean" | "japanese" | "chinese";

export interface Theme {
    colors: Map<string, number>;
}

export type PointerStyle = "default" | "text" | "pointer";

export interface Size {
    width: number;
    height: number;
}

export interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface Point {
    x: number;
    y: number;
}

export type ExternalElementData = { type: "image"; src: string; originalWidth: number; originalHeight: number };

export interface ExternalElement {
    pageIdx: number;
    nodeId: string;
    bounds: Rect;
    data: ExternalElementData;
    isSelected: boolean;
}

export type Cmd = { type: "docChanged" } | { type: "settingsChanged"; paragraphIndent: number; blockGap: number } | { type: "layoutChanged"; pageCount: number; layoutMode: LayoutMode; pageWidth: number; pageHeights: number[] } | { type: "cursorChanged"; pageIdx: number | undefined; bounds: Rect | undefined; show: boolean } | { type: "externalElementChanged"; elements: ExternalElement[] } | { type: "pointerStyleChanged"; style: PointerStyle } | { type: "selectionChanged"; stats: SelectionStats } | { type: "activeMarksChanged"; uniformMarks: Mark[]; mixedMarks: MarkType[] } | { type: "fontsRequired"; fonts: [string, number][] } | { type: "writingSystemRequired"; systems: WritingSystem[] } | { type: "renderRequired" } | { type: "enabledActionsChanged"; enabled: string[] };

export interface SelectionStats {
    blockCount: number;
    paragraphCount: number;
    uniformAlign: TextAlign | undefined;
    uniformLineHeight: number | undefined;
}

export type DropIndicator = { type: "inline"; pageIdx: number; x: number; y: number; height: number } | { type: "block"; pageIdx: number; x: number; y: number; width: number };

export type Direction = "left" | "right" | "up" | "down" | "lineStart" | "lineEnd" | "wordLeft" | "wordRight" | "documentStart" | "documentEnd";

export type Message = { type: "initialize"; theme: Theme } | { type: "input"; text: string } | { type: "paste"; fragment: string | undefined; html: string | undefined; text: string } | { type: "compositionStart"; text: string } | { type: "compositionUpdate"; text: string } | { type: "compositionEnd" } | { type: "pointerDown"; pageIdx: number; x: number; y: number; clickCount: number; shiftKey: boolean; isPrimary: boolean } | { type: "pointerMove"; pageIdx: number; x: number; y: number; isPressed: boolean } | { type: "pointerUp"; pageIdx: number; x: number; y: number } | { type: "dragStart"; pageIdx: number; x: number; y: number } | { type: "dragOver"; pageIdx: number; x: number; y: number } | { type: "dragEnter" } | { type: "dragLeave" } | { type: "drop"; pageIdx: number; x: number; y: number; text: string | undefined; html: string | undefined; fragment: string | undefined } | { type: "dragEnd" } | { type: "navigate"; direction: Direction; extend: boolean } | { type: "selectAll" } | { type: "deleteSelection" } | { type: "deleteBackward" } | { type: "deleteForward" } | { type: "deleteWordBackward" } | { type: "deleteWordForward" } | { type: "deleteToLineStart" } | { type: "insertNewline" } | { type: "insertHardBreak" } | { type: "insertPageBreak" } | { type: "toggleBold" } | { type: "toggleItalic" } | { type: "toggleStrikethrough" } | { type: "toggleUnderline" } | { type: "toggleRuby"; text: string } | { type: "toggleBlockquote" } | { type: "toggleCallout" } | { type: "toggleBulletList" } | { type: "toggleOrderedList" } | { type: "undo" } | { type: "redo" } | { type: "setFontFamily"; family: string } | { type: "setFontSize"; size: number } | { type: "setFontWeight"; weight: number } | { type: "setLineHeight"; height: number } | { type: "setLetterSpacing"; spacing: number } | { type: "setTextAlign"; align: TextAlign } | { type: "setBlockGap"; gap: number } | { type: "setParagraphIndent"; indent: number } | { type: "toggleTextColor"; key: string } | { type: "toggleBackgroundColor"; key: string | undefined } | { type: "clearFormatting" } | { type: "indent" } | { type: "outdent" } | { type: "extendMarkRange"; mark: Mark } | { type: "insertImage"; src: string; width: number; height: number } | { type: "insertHorizontalRule"; variant: HorizontalRuleVariant } | { type: "setLayoutMode"; mode: LayoutMode } | { type: "resize"; width: number; scaleFactor: number } | { type: "setTheme"; theme: Theme } | { type: "fontsLoaded" } | { type: "escape" } | { type: "toggleFoldExpansion"; nodeId: string } | { type: "insertFold" };

export class Application {
  free(): void;
  [Symbol.dispose](): void;
  createEditor(scale_factor: number, snapshot?: Uint8Array | null): Editor;
  loadIcuData(icu_data: Uint8Array): void;
  registerFont(name: string, weight: number, data: Uint8Array): void;
  setAvailableFonts(fonts: any): void;
  registerFallbackFont(name: string, weight: number, data: Uint8Array): void;
  constructor();
}
export class ClipboardData {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  fragment: string;
  html: string;
  text: string;
}
export class DragImageInfo {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  readonly scaleFactor: number;
  readonly len: number;
  readonly ptr: number;
  readonly width: number;
  readonly height: number;
  readonly offsetX: number;
  readonly offsetY: number;
}
export class Editor {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  canDragAt(page_idx: number, x: number, y: number): boolean;
  getVersion(): Uint8Array;
  renderPage(page_index: number): RenderInfo | undefined;
  getSnapshot(): Uint8Array;
  inspectState(): string;
  importUpdates(updates: Uint8Array): void;
  enqueueMessage(val: any): void;
  renderDragImage(visible_pages: Uint32Array, page_idx: number): DragImageInfo | undefined;
  exportAllUpdates(): Uint8Array;
  getClipboardData(): ClipboardData | undefined;
  exportUpdatesFrom(version: Uint8Array): Uint8Array;
  importUpdatesBatch(updates_batch: Array<any>): void;
  inspectPageElement(page_idx: number, x: number, y: number): string | undefined;
  inspectStateAsMacro(): string;
  inspectSelectionAsFragmentMacro(): string | undefined;
  tick(): any;
  flush(): void;
  dispatch(val: any): any;
}
export class RenderInfo {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  ptr: number;
  len: number;
  width: number;
  height: number;
}
