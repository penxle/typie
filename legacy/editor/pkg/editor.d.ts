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

export interface FontMapping {
    family: string;
    weight: number;
    codepoints: number[];
}

export interface FontMetadata {
    weight: number;
    style: string;
    names: FontName[];
}

export interface FontName {
    nameId: number;
    platformId: number;
    languageId: number;
    value: string;
}

export interface FontSizeStyle {
    /**
     * pt × 100 (e.g. 16pt → 1600)
     */
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
    /**
     * em × 100 (e.g. 0.05em → 5)
     */
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
    /**
     * × 100 (e.g. 160% → 160)
     */
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

export type Message = { type: "initialize"; theme: Theme; viewportWidth: number; viewportHeight: number; scaleFactor: number } | { type: "input"; text: string } | { type: "replaceBackward"; length: number; text: string } | { type: "pasteHtml"; html: string; text: string } | { type: "pasteHtmlAsText"; html: string; text: string } | { type: "pasteText"; text: string } | { type: "repasteAsText" } | { type: "compositionStart"; text: string } | { type: "compositionUpdate"; text: string; replaceLength?: number | undefined } | { type: "compositionEnd" } | { type: "commitPreedit" } | { type: "pointerDown"; pageIdx: number; x: number; y: number; clickCount: number; button: PointerButton; modifier: Modifier } | { type: "pointerMove"; pageIdx: number; x: number; y: number; buttons: number; modifier: Modifier } | { type: "pointerUp"; pageIdx: number; x: number; y: number; button: PointerButton; modifier: Modifier } | { type: "dragStart"; pageIdx: number; x: number; y: number } | { type: "dragOver"; pageIdx: number; x: number; y: number } | { type: "dragEnter" } | { type: "dragLeave" } | { type: "drop"; pageIdx: number; x: number; y: number; text: string | undefined; html: string | undefined; modifier: Modifier } | { type: "dropImages"; pageIdx: number; x: number; y: number; uploadIds: string[] } | { type: "dropFiles"; pageIdx: number; x: number; y: number; uploadIds: string[] } | { type: "dragEnd" } | { type: "navigate"; direction: Direction; extend: boolean } | { type: "selectAll" } | { type: "selectWord" } | { type: "selectSentence" } | { type: "selectParagraph" } | { type: "deleteSelection" } | { type: "deleteBackward"; length?: number | undefined } | { type: "deleteForward" } | { type: "deleteWordBackward" } | { type: "deleteWordForward" } | { type: "deleteSentenceBackward" } | { type: "deleteToLineStart" } | { type: "insertNewline" } | { type: "insertHardBreak" } | { type: "insertPageBreak" } | { type: "toggleBold" } | { type: "toggleStyle"; style: Style } | { type: "addAnnotation"; annotation: Annotation } | { type: "updateAnnotation"; annotation: Annotation } | { type: "removeAnnotation"; annotationType: AnnotationType } | { type: "toggleBlockquote"; variant: BlockquoteVariant } | { type: "setBlockquote"; variant: BlockquoteVariant } | { type: "toggleCallout" } | { type: "cycleCalloutVariant" } | { type: "toggleBulletList" } | { type: "toggleOrderedList" } | { type: "undo" } | { type: "redo" } | { type: "setLineHeight"; height: number } | { type: "setTextAlign"; align: TextAlign } | { type: "setBlockGap"; gap: number } | { type: "setParagraphIndent"; indent: number } | { type: "setDefaultAttrs"; attrs: DefaultAttrs } | { type: "clearFormatting" } | { type: "indent" } | { type: "outdent" } | { type: "insertImage"; uploadId: string | undefined } | { type: "insertFile"; uploadId: string | undefined } | { type: "insertEmbed" } | { type: "insertHorizontalRule"; variant: HorizontalRuleVariant } | { type: "setHorizontalRule"; variant: HorizontalRuleVariant } | { type: "setLayoutMode"; mode: LayoutMode } | { type: "resize"; width: number; height: number; scaleFactor: number } | { type: "setTheme"; theme: Theme } | { type: "fontsLoaded"; family: string; weight: number; mappings: FontMapping[] } | { type: "escape" } | { type: "insertFold" } | { type: "unwrapFold" } | { type: "insertTable"; rows: number; cols: number } | { type: "setColumnWidths"; tableId: string; colWidths: number[] } | { type: "addTableRow"; tableId: string; row: number; before: boolean } | { type: "addTableColumn"; tableId: string; col: number; before: boolean } | { type: "deleteTableRow"; tableId: string; row: number } | { type: "deleteTableColumn"; tableId: string; col: number } | { type: "setTableBorderStyle"; tableId: string; style: string } | { type: "setTableAlign"; tableId: string; align: TableAlign } | { type: "setTableProportion"; tableId: string; proportion: number } | { type: "setTableWidth"; tableId: string; width: number; contentWidth: number } | { type: "selectTable"; tableId: string } | { type: "selectTableRow"; tableId: string; row: number } | { type: "selectTableColumn"; tableId: string; col: number } | { type: "moveTableRow"; tableId: string; fromRow: number; toRow: number } | { type: "moveTableColumn"; tableId: string; fromCol: number; toCol: number } | { type: "deleteNode"; nodeId: string } | { type: "setImageProportion"; nodeId: string; proportion: number } | { type: "setImageId"; nodeId: string; imageId: string } | { type: "setFileId"; nodeId: string; fileId: string } | { type: "setEmbedId"; nodeId: string; embedId: string } | { type: "setExternalElementHeight"; nodeId: string; height: number } | { type: "setFocused"; focused: boolean } | { type: "setSelection"; anchorNodeId: string; anchorOffset: number; anchorAffinity: Affinity; headNodeId: string; headOffset: number; headAffinity: Affinity } | { type: "collapseSelection"; toAnchor: boolean } | { type: "extendSelectionTo"; anchorPageIdx: number; anchorX: number; anchorY: number; headPageIdx: number; headX: number; headY: number; doubleTapInitialRange: Selection | undefined } | { type: "addRemark"; nodeId: string; userId: string; text: string; createdAt: number } | { type: "updateRemark"; nodeId: string; remarkId: string; text: string } | { type: "removeRemark"; nodeId: string; remarkId: string } | { type: "toggleFold"; nodeId: string } | { type: "cycleCalloutVariantAt"; nodeId: string };

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
    clearTracing(): void;
    dispatch(val: any): void;
    drainTraces(): any;
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
    isCursorHit(page_idx: number, x: number, y: number): boolean;
    isDetached(): boolean;
    isReadOnly(): boolean;
    isSelectionHit(page_idx: number, x: number, y: number): boolean;
    performSearch(query: string, match_whole_word: boolean): any;
    removeTrackedItems(group: number, ids: string[]): void;
    renderDragImage(visible_pages: Uint32Array, page_idx: number): DragImageInfo | undefined;
    renderPage(page_index: number): RenderInfo | undefined;
    replaceTextInBlock(block_id: string, start_offset: number, end_offset: number, replacement: string): boolean;
    replaceTextInBlocks(items: any): boolean;
    revealTrackedItem(group: number, id: string): boolean;
    revertTo(version: Uint8Array): void;
    setAllFoldsExpanded(expanded: boolean): void;
    setLayoutDebug(enabled: boolean): void;
    setMaxPages(max_pages?: number | null): void;
    setReadOnly(read_only: boolean): void;
    setRenderDebug(enabled: boolean): void;
    setTracing(trace_id: string, parent_span_id: string): void;
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

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_application_free: (a: number, b: number) => void;
    readonly __wbg_charactercounts_free: (a: number, b: number) => void;
    readonly __wbg_clipboarddata_free: (a: number, b: number) => void;
    readonly __wbg_dragimageinfo_free: (a: number, b: number) => void;
    readonly __wbg_editor_free: (a: number, b: number) => void;
    readonly __wbg_get_charactercounts_doc_with_whitespace: (a: number) => number;
    readonly __wbg_get_charactercounts_doc_without_whitespace: (a: number) => number;
    readonly __wbg_get_charactercounts_doc_without_whitespace_and_punctuation: (a: number) => number;
    readonly __wbg_get_charactercounts_selection_with_whitespace: (a: number) => number;
    readonly __wbg_get_charactercounts_selection_without_whitespace: (a: number) => number;
    readonly __wbg_get_charactercounts_selection_without_whitespace_and_punctuation: (a: number) => number;
    readonly __wbg_get_clipboarddata_html: (a: number) => [number, number];
    readonly __wbg_get_clipboarddata_text: (a: number) => [number, number];
    readonly __wbg_renderinfo_free: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_doc_with_whitespace: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_doc_without_whitespace: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_doc_without_whitespace_and_punctuation: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_selection_with_whitespace: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_selection_without_whitespace: (a: number, b: number) => void;
    readonly __wbg_set_charactercounts_selection_without_whitespace_and_punctuation: (a: number, b: number) => void;
    readonly __wbg_set_clipboarddata_html: (a: number, b: number, c: number) => void;
    readonly __wbg_set_clipboarddata_text: (a: number, b: number, c: number) => void;
    readonly application_addFontBase: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly application_addFontChunk: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly application_clearTextReplacementRules: (a: number) => void;
    readonly application_createEditor: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly application_encodeFont: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly application_getFontCodepoints: (a: number, b: number, c: number) => [number, number, number];
    readonly application_getFontMetadata: (a: number, b: number, c: number) => [number, number, number];
    readonly application_getMemory: (a: number) => any;
    readonly application_jsonToSnapshot: (a: number, b: any) => [number, number, number, number];
    readonly application_loadIcuData: (a: number, b: number, c: number) => [number, number];
    readonly application_new: () => number;
    readonly application_outlineTextToSvg: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly application_setAutoSurroundEnabled: (a: number, b: number) => void;
    readonly application_setAvailableFonts: (a: number, b: any) => void;
    readonly application_setTextReplacementRules: (a: number, b: any) => void;
    readonly application_snapshotToJson: (a: number, b: number, c: number) => [number, number, number];
    readonly application_validateDocumentJson: (a: number, b: any) => [number, number];
    readonly application_validateRegex: (a: number, b: number, c: number) => number;
    readonly dragimageinfo_height: (a: number) => number;
    readonly dragimageinfo_len: (a: number) => number;
    readonly dragimageinfo_offsetX: (a: number) => number;
    readonly dragimageinfo_offsetY: (a: number) => number;
    readonly dragimageinfo_ptr: (a: number) => number;
    readonly dragimageinfo_scaleFactor: (a: number) => number;
    readonly dragimageinfo_width: (a: number) => number;
    readonly editor_checkout: (a: number, b: number, c: number) => [number, number];
    readonly editor_checkoutToLatest: (a: number) => [number, number];
    readonly editor_clearTracing: (a: number) => void;
    readonly editor_dispatch: (a: number, b: any) => void;
    readonly editor_drainTraces: (a: number) => any;
    readonly editor_enqueueMessage: (a: number, b: any) => void;
    readonly editor_export: (a: number, b: any) => [number, number, number, number];
    readonly editor_exportPageVector: (a: number, b: number) => [number, number];
    readonly editor_flush: (a: number) => void;
    readonly editor_getCharacterCountAtVersion: (a: number, b: number, c: number) => number;
    readonly editor_getCharacterCounts: (a: number) => number;
    readonly editor_getClipboardData: (a: number) => number;
    readonly editor_getSlabLen: (a: number) => number;
    readonly editor_getSlabPtr: (a: number) => number;
    readonly editor_getSlateLen: (a: number) => number;
    readonly editor_getSlateOffsets: (a: number) => any;
    readonly editor_getSlatePtr: (a: number) => number;
    readonly editor_getTextWithMappings: (a: number) => [number, number, number];
    readonly editor_importUpdates: (a: number, b: number, c: number) => [number, number];
    readonly editor_importUpdatesBatch: (a: number, b: any) => [number, number];
    readonly editor_insertTemplateFragment: (a: number, b: number, c: number) => [number, number];
    readonly editor_inspectPageElement: (a: number, b: number, c: number, d: number) => [number, number];
    readonly editor_inspectSelectionAsFragmentMacro: (a: number) => [number, number];
    readonly editor_inspectState: (a: number) => [number, number];
    readonly editor_inspectStateAsMacro: (a: number) => [number, number];
    readonly editor_isCursorHit: (a: number, b: number, c: number, d: number) => number;
    readonly editor_isDetached: (a: number) => number;
    readonly editor_isReadOnly: (a: number) => number;
    readonly editor_isSelectionHit: (a: number, b: number, c: number, d: number) => number;
    readonly editor_performSearch: (a: number, b: number, c: number, d: number) => any;
    readonly editor_removeTrackedItems: (a: number, b: number, c: number, d: number) => void;
    readonly editor_renderDragImage: (a: number, b: number, c: number, d: number) => number;
    readonly editor_renderPage: (a: number, b: number) => number;
    readonly editor_replaceTextInBlock: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly editor_replaceTextInBlocks: (a: number, b: any) => number;
    readonly editor_revealTrackedItem: (a: number, b: number, c: number, d: number) => number;
    readonly editor_revertTo: (a: number, b: number, c: number) => [number, number];
    readonly editor_setAllFoldsExpanded: (a: number, b: number) => void;
    readonly editor_setLayoutDebug: (a: number, b: number) => void;
    readonly editor_setMaxPages: (a: number, b: number) => void;
    readonly editor_setReadOnly: (a: number, b: number) => void;
    readonly editor_setRenderDebug: (a: number, b: number) => void;
    readonly editor_setTracing: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly editor_setTrackedItems: (a: number, b: number, c: number, d: number) => void;
    readonly editor_tick: (a: number) => void;
    readonly __wbg_get_renderinfo_height: (a: number) => number;
    readonly __wbg_get_renderinfo_len: (a: number) => number;
    readonly __wbg_get_renderinfo_ptr: (a: number) => number;
    readonly __wbg_get_renderinfo_width: (a: number) => number;
    readonly __wbg_set_renderinfo_height: (a: number, b: number) => void;
    readonly __wbg_set_renderinfo_len: (a: number, b: number) => void;
    readonly __wbg_set_renderinfo_ptr: (a: number, b: number) => void;
    readonly __wbg_set_renderinfo_width: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
