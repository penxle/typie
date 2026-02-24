/* tslint:disable */
/* eslint-disable */
export interface ArchivedNode {
    id: string | undefined;
}

export interface BackgroundColorStyle {
    color: string;
}

export interface BlockquoteNode {
    variant?: BlockquoteVariant;
}

export interface BoldStyle {}

export interface BulletListNode {}

export interface CalloutNode {
    variant?: CalloutVariant;
}

export interface EmbedNode {
    id: string | undefined;
}

export interface EncodedFont {
    base: Uint8Array;
    chunks: Uint8Array[];
}

export interface FileNode {
    id: string | undefined;
}

export interface FoldContentNode {}

export interface FoldNode {}

export interface FoldTitleNode {}

export interface FontFamilyStyle {
    family: string;
}

export interface FontMetadata {
    weight: number;
    style: string;
    familyName: string | undefined;
    displayName: string | undefined;
    fullName: string | undefined;
    postScriptName: string;
    subfamilyDisplayName: string | undefined;
}

export interface FontSizeStyle {
    size: number;
}

export interface FontWeightStyle {
    weight: number;
}

export interface HardBreakNode {}

export interface HorizontalRuleNode {
    variant?: HorizontalRuleVariant;
}

export interface ImageNode {
    id: string | undefined;
    proportion?: number;
}

export interface ItalicStyle {}

export interface LetterSpacingStyle {
    spacing: number;
}

export interface LinkAnnotation {
    href: string;
}

export interface ListItemNode {}

export interface Modifier {
    shift: boolean;
    ctrl: boolean;
    alt: boolean;
    meta: boolean;
}

export interface OrderedListNode {}

export interface PageBreakNode {}

export interface ParagraphNode {
    align?: TextAlign;
    line_height?: number;
}

export interface Point {
    x: number;
    y: number;
}

export interface Position {
    nodeId: NodeId;
    offset: number;
    affinity: Affinity;
}

export interface RawTrackedItem {
    id: string;
    nodeId: NodeId;
    startOffset: number;
    endOffset: number;
}

export interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface Remark {
    id: RemarkId;
    userId: string;
    text: string;
    createdAt: number;
}

export interface RootNode {}

export interface RubyAnnotation {
    text: string;
}

export interface Selection {
    anchor: Position;
    head: Position;
}

export interface Size {
    width: number;
    height: number;
}

export interface StrikethroughStyle {}

export interface TableCellNode {
    col_width?: number | undefined;
}

export interface TableNode {
    border_style?: TableBorderStyle;
    align?: TableAlign;
    proportion?: number;
}

export interface TableRowNode {}

export interface TextBound {
    x: number;
    y: number;
    width: number;
    height: number;
    ascent: number;
}

export interface TextColorStyle {
    color: string;
}

export interface TextNode {
    text: Text;
}

export interface Theme {
    colors: FxHashMap<string, number>;
}

export interface UnderlineStyle {}

export type Affinity = "upstream" | "downstream";

export type Annotation = ({ type: "link" } & LinkAnnotation) | ({ type: "ruby" } & RubyAnnotation);

export type AnnotationType = "link" | "ruby";

export type Attr = ({ attr: "style" } & Style) | ({ attr: "paragraph" } & ParagraphAttr);

export type BlockquoteVariant = "left_line" | "left_quote" | "message_sent" | "message_received";

export type CalloutVariant = "info" | "success" | "warning" | "danger";

export type Codepoints = number[];

export type DefaultAttrs = Attr[];

export type Direction = "left" | "right" | "up" | "down" | "lineStart" | "lineEnd" | "wordLeft" | "wordRight" | "documentStart" | "documentEnd" | "pageUp" | "pageDown" | "sentenceUp" | "sentenceDown";

export type DocExportMode = { type: "snapshot" } | { type: "version" } | { type: "all-updates" } | { type: "updates-from"; version: Uint8Array };

export type DropIndicator = { type: "inline"; pageIdx: number; x: number; y: number; height: number } | { type: "block"; pageIdx: number; x: number; y: number; width: number };

export type ExternalElementData = { type: "image"; id: string | undefined; proportion: number; uploadId: string | undefined } | { type: "file"; id: string | undefined; uploadId: string | undefined } | { type: "embed"; id: string | undefined } | { type: "archived"; id: string | undefined };

export type HorizontalRuleVariant = "line" | "dashed_line" | "circle_line" | "diamond_line" | "circle" | "diamond" | "three_circles" | "three_diamonds" | "zigzag";

export type LayoutMode = { type: "paginated"; pageWidth: number; pageHeight: number; pageMarginTop: number; pageMarginBottom: number; pageMarginLeft: number; pageMarginRight: number } | { type: "continuous"; maxWidth: number };

export type Message = { type: "initialize"; theme: Theme } | { type: "input"; text: string } | { type: "replaceBackward"; length: number; text: string } | { type: "pasteHtml"; html: string; text: string } | { type: "pasteText"; text: string } | { type: "repasteAsText" } | { type: "compositionStart"; text: string } | { type: "compositionUpdate"; text: string } | { type: "compositionEnd" } | { type: "commitPreedit" } | { type: "pointerDown"; pageIdx: number; x: number; y: number; clickCount: number; button: PointerButton; modifier: Modifier } | { type: "pointerMove"; pageIdx: number; x: number; y: number; buttons: number; modifier: Modifier } | { type: "pointerUp"; pageIdx: number; x: number; y: number; button: PointerButton; modifier: Modifier } | { type: "dragStart"; pageIdx: number; x: number; y: number } | { type: "dragOver"; pageIdx: number; x: number; y: number } | { type: "dragEnter" } | { type: "dragLeave" } | { type: "drop"; pageIdx: number; x: number; y: number; text: string | undefined; html: string | undefined; modifier: Modifier } | { type: "dropImages"; pageIdx: number; x: number; y: number; uploadIds: string[] } | { type: "dropFiles"; pageIdx: number; x: number; y: number; uploadIds: string[] } | { type: "dragEnd" } | { type: "navigate"; direction: Direction; extend: boolean } | { type: "selectAll" } | { type: "selectWord" } | { type: "selectSentence" } | { type: "selectParagraph" } | { type: "deleteSelection" } | { type: "deleteBackward" } | { type: "deleteForward" } | { type: "deleteWordBackward" } | { type: "deleteWordForward" } | { type: "deleteSentenceBackward" } | { type: "deleteToLineStart" } | { type: "insertNewline" } | { type: "insertHardBreak" } | { type: "insertPageBreak" } | { type: "toggleBold" } | { type: "toggleStyle"; style: Style } | { type: "addAnnotation"; annotation: Annotation } | { type: "updateAnnotation"; annotation: Annotation } | { type: "removeAnnotation"; annotationType: AnnotationType } | { type: "toggleBlockquote"; variant: BlockquoteVariant } | { type: "setBlockquote"; variant: BlockquoteVariant } | { type: "toggleCallout" } | { type: "cycleCalloutVariant" } | { type: "toggleBulletList" } | { type: "toggleOrderedList" } | { type: "undo" } | { type: "redo" } | { type: "setLineHeight"; height: number } | { type: "setTextAlign"; align: TextAlign } | { type: "setBlockGap"; gap: number } | { type: "setParagraphIndent"; indent: number } | { type: "setDefaultAttrs"; attrs: DefaultAttrs } | { type: "clearFormatting" } | { type: "indent" } | { type: "outdent" } | { type: "insertImage"; uploadId: string | undefined } | { type: "insertFile"; uploadId: string | undefined } | { type: "insertEmbed" } | { type: "insertHorizontalRule"; variant: HorizontalRuleVariant } | { type: "setHorizontalRule"; variant: HorizontalRuleVariant } | { type: "setLayoutMode"; mode: LayoutMode } | { type: "resize"; width: number; height: number; scaleFactor: number } | { type: "setTheme"; theme: Theme } | { type: "fontsLoaded"; family: string; weight: number } | { type: "escape" } | { type: "insertFold" } | { type: "unwrapFold" } | { type: "insertTable"; rows: number; cols: number } | { type: "setColumnWidths"; tableId: string; colWidths: number[] } | { type: "addTableRow"; tableId: string; row: number; before: boolean } | { type: "addTableColumn"; tableId: string; col: number; before: boolean } | { type: "deleteTableRow"; tableId: string; row: number } | { type: "deleteTableColumn"; tableId: string; col: number } | { type: "setTableBorderStyle"; tableId: string; style: string } | { type: "setTableAlign"; tableId: string; align: TableAlign } | { type: "setTableProportion"; tableId: string; proportion: number } | { type: "setTableWidth"; tableId: string; width: number; contentWidth: number } | { type: "selectTable"; tableId: string } | { type: "selectTableRow"; tableId: string; row: number } | { type: "selectTableColumn"; tableId: string; col: number } | { type: "moveTableRow"; tableId: string; fromRow: number; toRow: number } | { type: "moveTableColumn"; tableId: string; fromCol: number; toCol: number } | { type: "deleteNode"; nodeId: string } | { type: "setImageProportion"; nodeId: string; proportion: number } | { type: "setImageId"; nodeId: string; imageId: string } | { type: "setFileId"; nodeId: string; fileId: string } | { type: "setEmbedId"; nodeId: string; embedId: string } | { type: "setExternalElementHeight"; nodeId: string; height: number } | { type: "setFocused"; focused: boolean } | { type: "setSelection"; anchorNodeId: string; anchorOffset: number; anchorAffinity: Affinity; headNodeId: string; headOffset: number; headAffinity: Affinity } | { type: "collapseSelection"; toAnchor: boolean } | { type: "extendSelectionTo"; anchorPageIdx: number; anchorX: number; anchorY: number; headPageIdx: number; headX: number; headY: number; doubleTapInitialRange: Selection | undefined } | { type: "addRemark"; nodeId: string; userId: string; text: string; createdAt: number } | { type: "updateRemark"; nodeId: string; remarkId: string; text: string } | { type: "removeRemark"; nodeId: string; remarkId: string };

export type Node = ({ type: "root" } & RootNode) | ({ type: "paragraph" } & ParagraphNode) | ({ type: "blockquote" } & BlockquoteNode) | ({ type: "callout" } & CalloutNode) | ({ type: "image" } & ImageNode) | ({ type: "file" } & FileNode) | ({ type: "embed" } & EmbedNode) | ({ type: "archived" } & ArchivedNode) | ({ type: "text" } & TextNode) | ({ type: "hard_break" } & HardBreakNode) | ({ type: "horizontal_rule" } & HorizontalRuleNode) | ({ type: "page_break" } & PageBreakNode) | ({ type: "bullet_list" } & BulletListNode) | ({ type: "ordered_list" } & OrderedListNode) | ({ type: "list_item" } & ListItemNode) | ({ type: "fold" } & FoldNode) | ({ type: "fold_title" } & FoldTitleNode) | ({ type: "fold_content" } & FoldContentNode) | ({ type: "table" } & TableNode) | ({ type: "table_row" } & TableRowNode) | ({ type: "table_cell" } & TableCellNode);

export type NodeType = "root" | "paragraph" | "blockquote" | "callout" | "text" | "image" | "file" | "embed" | "archived" | "hard_break" | "horizontal_rule" | "page_break" | "bullet_list" | "ordered_list" | "list_item" | "fold" | "fold_title" | "fold_content" | "table" | "table_row" | "table_cell";

export type PointerButton = "primary" | "auxiliary" | "secondary";

export type PointerStyle = "default" | "text" | "pointer";

export type Style = ({ type: "background_color" } & BackgroundColorStyle) | ({ type: "bold" } & BoldStyle) | ({ type: "text_color" } & TextColorStyle) | ({ type: "font_size" } & FontSizeStyle) | ({ type: "font_family" } & FontFamilyStyle) | ({ type: "font_weight" } & FontWeightStyle) | ({ type: "italic" } & ItalicStyle) | ({ type: "letter_spacing" } & LetterSpacingStyle) | ({ type: "strikethrough" } & StrikethroughStyle) | ({ type: "underline" } & UnderlineStyle);

export type StyleType = "background_color" | "bold" | "text_color" | "font_size" | "font_family" | "font_weight" | "italic" | "letter_spacing" | "strikethrough" | "underline";

export type TableAlign = "left" | "center" | "right";

export type TableBorderStyle = "solid" | "dashed" | "dotted" | "none";

export type TextAlign = "left" | "center" | "right" | "justify";

export type TrackedItemGroup = "spellcheck" | "aiFeedback" | "search";


export class Application {
    free(): void;
    [Symbol.dispose](): void;
    addFontBase(family: string, weight: number, data: Uint8Array): void;
    addFontChunk(family: string, weight: number, data: Uint8Array): void;
    clearTextReplacementRules(): void;
    createEditor(scale_factor: number, snapshot?: Uint8Array | null): Editor;
    encodeFont(ttf_data: Uint8Array, chunk_codepoints_json: string): EncodedFont;
    getFontCodepoints(ttf_data: Uint8Array): Codepoints;
    getFontMetadata(data: Uint8Array): FontMetadata;
    getMemory(): any;
    jsonToSnapshot(json: any): Uint8Array;
    loadIcuData(icu_data: Uint8Array): void;
    constructor();
    outlineTextToSvg(font_data: Uint8Array, text: string): string;
    setAutoSurroundEnabled(enabled: boolean): void;
    setAvailableFonts(fonts: any): void;
    setFallbackFonts(names: any): void;
    setTextReplacementRules(rules: any): void;
    snapshotToJson(snapshot: Uint8Array): any;
    validateDocumentJson(json: any): void;
    validateRegex(pattern: string): boolean;
}

export class CharacterCounts {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    doc_with_whitespace: number;
    doc_without_whitespace_and_punctuation: number;
    doc_without_whitespace: number;
    selection_with_whitespace: number;
    selection_without_whitespace_and_punctuation: number;
    selection_without_whitespace: number;
}

export class ClipboardData {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    html: string;
    text: string;
}

export class DragImageInfo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly height: number;
    readonly len: number;
    readonly offsetX: number;
    readonly offsetY: number;
    readonly ptr: number;
    readonly scaleFactor: number;
    readonly width: number;
}

export class Editor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    checkout(version: Uint8Array): void;
    checkoutToLatest(): void;
    dispatch(val: any): void;
    enqueueMessage(val: any): void;
    export(mode: DocExportMode): Uint8Array;
    exportPageVector(page_index: number): Uint8Array | undefined;
    flush(): void;
    getCharacterCountAtVersion(version: Uint8Array): number | undefined;
    getCharacterCounts(): CharacterCounts;
    getClipboardData(): ClipboardData | undefined;
    getSlabLen(): number;
    getSlabPtr(): number;
    getSlateLen(): number;
    getSlateOffsets(): any;
    getSlatePtr(): number;
    getTextWithMappings(): any;
    importUpdates(updates: Uint8Array): void;
    importUpdatesBatch(updates_batch: Array<any>): void;
    insertTemplateFragment(snapshot: Uint8Array): void;
    inspectPageElement(page_idx: number, x: number, y: number): string | undefined;
    inspectSelectionAsFragmentMacro(): string | undefined;
    inspectState(): string;
    inspectStateAsMacro(): string;
    isDetached(): boolean;
    isReadOnly(): boolean;
    isSelectionHit(page_idx: number, x: number, y: number): boolean;
    performSearch(query: string, match_whole_word: boolean): any;
    removeTrackedItems(group: number, ids: string[]): void;
    renderDragImage(visible_pages: Uint32Array, page_idx: number): DragImageInfo | undefined;
    renderPage(page_index: number): RenderInfo | undefined;
    replaceTextInBlock(block_id: string, start_offset: number, end_offset: number, replacement: string): boolean;
    replaceTextInBlocks(items: any): boolean;
    revertTo(version: Uint8Array): void;
    setAllFoldsExpanded(expanded: boolean): void;
    setLayoutDebug(enabled: boolean): void;
    setReadOnly(read_only: boolean): void;
    setRenderDebug(enabled: boolean): void;
    setTrackedItems(group: number, raw_items: RawTrackedItem[]): void;
    tick(): void;
}

export class RenderInfo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    height: number;
    len: number;
    ptr: number;
    width: number;
}
