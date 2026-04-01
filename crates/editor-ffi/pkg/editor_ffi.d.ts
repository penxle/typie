/* tslint:disable */
/* eslint-disable */
export interface ArchivedNode {
    id: string | undefined;
}

export interface BlockquoteNode {
    variant: BlockquoteVariant;
}

export interface BulletListNode {}

export interface CalloutNode {
    variant: CalloutVariant;
}

export interface EmbedNode {
    id: string | undefined;
}

export interface FileNode {
    id: string | undefined;
}

export interface FoldContentNode {}

export interface FoldNode {}

export interface FoldTitleNode {}

export interface FontMapping {
    family: string;
    weight: number;
    codepoints: number[];
}

export interface HardBreakNode {}

export interface HorizontalRuleNode {
    variant: HorizontalRuleVariant;
}

export interface ImageNode {
    id: string | undefined;
    proportion: number;
}

export interface KeyEvent {
    key: Key;
    modifiers: KeyModifiers;
}

export interface KeyModifiers {
    shift: boolean;
    ctrl: boolean;
    alt: boolean;
    meta: boolean;
}

export interface ListItemNode {}

export interface OrderedListNode {}

export interface PageBreakNode {}

export interface ParagraphNode {
    align: TextAlign;
}

export interface Position {
    node_id: NodeId;
    offset: number;
    affinity: Affinity;
}

export interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface RootNode {}

export interface Selection {
    anchor: Position;
    head: Position;
}

export interface Size {
    width: number;
    height: number;
}

export interface TableCellNode {
    col_width: number | undefined;
}

export interface TableNode {
    border_style: TableBorderStyle;
    align: TableAlign;
    proportion: number;
}

export interface TableRowNode {}

export interface TextNode {
    text: string;
}

export interface Viewport {
    width: number;
    height: number;
    scale_factor: number;
}

export type Affinity = "Downstream" | "Upstream";

export type Axis = "Horizontal" | "Vertical";

export type BackendKind = "Cpu" | "Gpu";

export type BlockquoteVariant = "LeftLine" | "LeftQuote" | "MessageSent" | "MessageReceived";

export type BreakKind = "Block" | "Line" | "Page";

export type CalloutVariant = "Info" | "Success" | "Warning" | "Danger";

export type ClipboardIntent = { Paste: { html: string | undefined; text: string } } | "Cut" | "Copy";

export type CompositionIntent = { Update: { text: string; replace_length: number | undefined } } | "End";

export type DeletionIntent = "Selection" | { Move: Movement };

export type Direction = "Forward" | "Backward";

export type DragEvent = { Start: { x: number; y: number } } | { Over: { x: number; y: number } } | "Enter" | "Leave" | "End" | { Drop: { x: number; y: number; payload: DragPayload } };

export type DragPayload = "Internal" | { Text: string } | { Html: { html: string; text: string } } | { Files: string[] };

export type EditorEvent = { StateChanged: { fields: StateField[] } } | "DocumentChanged" | "RenderInvalidated" | { FontMissing: { family: string; weight: number } } | "CursorExitedDocumentStart";

export type Effect = { LoadFont: { family: string; weight: number; codepoints: number[] } };

export type FormattingIntent = { ToggleModifier: ModifierType } | { SetModifier: Modifier } | "Clear" | { SetTextAlign: TextAlign } | { SetLineHeight: number } | { ToggleWrap: NodeType } | "Indent" | "Outdent";

export type HistoryIntent = "Undo" | "Redo";

export type HorizontalRuleVariant = "Line" | "DashedLine" | "CircleLine" | "DiamondLine" | "Circle" | "Diamond" | "ThreeCircles" | "ThreeDiamonds" | "Zigzag";

export type InsertionIntent = { Text: string } | { Break: BreakKind } | { Block: Node };

export type Intent = { Insertion: InsertionIntent } | { Deletion: DeletionIntent } | { Formatting: FormattingIntent } | { Selection: SelectionIntent } | { Node: NodeIntent } | { Clipboard: ClipboardIntent } | { Composition: CompositionIntent } | { Navigation: NavigationIntent } | { History: HistoryIntent };

export type Key = "Enter" | "Backspace" | "Delete" | "Tab" | "Escape";

export type Message = { Key: KeyEvent } | { Pointer: PointerEvent } | { Intent: Intent } | { System: SystemEvent };

export type Modifier = "Bold" | "Italic" | "Underline" | "Strikethrough" | { FontSize: number } | { FontFamily: string } | { FontWeight: number } | { TextColor: string } | { BackgroundColor: string } | { LetterSpacing: number } | { Link: { href: string } } | { Ruby: { text: string } } | { LineHeight: number } | { BlockGap: number } | { ParagraphIndent: number };

export type ModifierType = "Bold" | "Italic" | "Underline" | "Strikethrough" | "FontSize" | "FontFamily" | "FontWeight" | "TextColor" | "BackgroundColor" | "LetterSpacing" | "Link" | "Ruby" | "LineHeight" | "BlockGap" | "ParagraphIndent";

export type Movement = { Grapheme: Direction } | { Word: Direction } | { Sentence: Direction } | { Line: [Direction, Axis] } | { Block: Direction } | { Page: Direction } | { Document: Direction };

export type NavigationIntent = { Move: { movement: Movement; extend: boolean } };

export type Node = { Root: RootNode } | { Paragraph: ParagraphNode } | { Blockquote: BlockquoteNode } | { Callout: CalloutNode } | { Text: TextNode } | { BulletList: BulletListNode } | { OrderedList: OrderedListNode } | { ListItem: ListItemNode } | { Fold: FoldNode } | { FoldTitle: FoldTitleNode } | { FoldContent: FoldContentNode } | { Table: TableNode } | { TableRow: TableRowNode } | { TableCell: TableCellNode } | { Image: ImageNode } | { File: FileNode } | { Embed: EmbedNode } | { Archived: ArchivedNode } | { HardBreak: HardBreakNode } | { HorizontalRule: HorizontalRuleNode } | { PageBreak: PageBreakNode };

export type NodeId = string;

export type NodeIntent = { Delete: { id: NodeId } } | { SetAttrs: { id: NodeId; attrs: Node } } | { ToggleFold: { id: NodeId } } | { Table: { id: NodeId; op: TableOp } };

export type NodeType = "Root" | "Paragraph" | "Blockquote" | "Callout" | "Text" | "BulletList" | "OrderedList" | "ListItem" | "Fold" | "FoldTitle" | "FoldContent" | "Table" | "TableRow" | "TableCell" | "Image" | "File" | "Embed" | "Archived" | "HardBreak" | "HorizontalRule" | "PageBreak";

export type PointerButton = "Primary" | "Auxiliary" | "Secondary";

export type PointerEvent = { Down: { x: number; y: number; count: number; button: PointerButton; modifiers: KeyModifiers } } | { Move: { x: number; y: number; buttons: number } } | { Up: { x: number; y: number; button: PointerButton } } | { Drag: DragEvent };

export type SelectionIntent = "All" | { Set: Selection };

export type StateField = "Selection" | "Cursor" | "Pages" | "Modifiers";

export type SystemEvent = "Initialize" | { Resize: { width: number; height: number; scale_factor: number } } | { SetFocused: boolean } | { FontsLoaded: { family: string; weight: number; mappings: FontMapping[] } } | { SetExternalHeight: { node_id: NodeId; height: number } };

export type TableAlign = "Left" | "Center" | "Right";

export type TableBorderStyle = "Solid" | "Dashed" | "Dotted" | "None";

export type TableOp = { InsertAxis: { axis: Axis; index: number; before: boolean } } | { DeleteAxis: { axis: Axis; index: number } } | { MoveAxis: { axis: Axis; from: number; to: number } } | { SelectAxis: Axis | undefined } | { SetColumnWidths: number[] };

export type TextAlign = "Left" | "Center" | "Right" | "Justify";


declare class Editor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    attach_surface(page: number, handle: HTMLCanvasElement, width: number, height: number, scale_factor: number): void;
    detach_surface(page: number): void;
    enqueue(message: Message): void;
    render_surface(page: number): void;
    resize_surface(page: number, width: number, height: number, scale_factor: number): void;
    selection(): Selection;
    tick(): EditorEvent[];
}

declare class EditorHost {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static create(kind: BackendKind): Promise<EditorHost>;
    create_editor(doc: string, viewport: Viewport): Editor;
    load_font_base(family: string, weight: number, data: Uint8Array): void;
    load_font_chunk(family: string, weight: number, data: Uint8Array): void;
    load_icu_data(data: Uint8Array): void;
    set_fallback_font_families(families: string[]): void;
}

export type { Editor, EditorHost };

export function createInstance(wasmModule: WebAssembly.Module): Promise<{
    Editor: typeof Editor;
    EditorHost: typeof EditorHost;
}>;
