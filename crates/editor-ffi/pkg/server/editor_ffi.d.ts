/* tslint:disable */
/* eslint-disable */
/**
 *Auto-generated discriminant enum variants
 */
export type ModifierType = "bold" | "italic" | "underline" | "strikethrough" | "font_size" | "font_family" | "font_weight" | "text_color" | "background_color" | "letter_spacing" | "link" | "ruby" | "line_height" | "block_gap" | "paragraph_indent";

/**
 *Auto-generated discriminant enum variants
 */
export type NodeType = "root" | "paragraph" | "blockquote" | "callout" | "text" | "bullet_list" | "ordered_list" | "list_item" | "fold" | "fold_title" | "fold_content" | "table" | "table_row" | "table_cell" | "image" | "file" | "embed" | "archived" | "hard_break" | "horizontal_rule" | "page_break";

export interface ArchivedNode {
    id: string | undefined;
}

export interface BlockquoteNode {
    variant?: BlockquoteVariant;
}

export interface BulletListNode {}

export interface CalloutNode {
    variant?: CalloutVariant;
}

export interface Doc {
    nodes: Record<NodeId, NodeEntry>;
    attrs: DocumentAttrs;
}

export interface DocumentAttrs {
    layout_mode: LayoutMode;
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

export interface FontFamily {
    name: string;
    weights: number[];
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

export interface HardBreakNode {}

export interface HorizontalRuleNode {
    variant?: HorizontalRuleVariant;
}

export interface ImageNode {
    id: string | undefined;
    proportion?: number;
}

export interface InputContext {
    text_before_cursor: string;
    text_after_cursor: string;
    selected_text: string;
    cursor_position: number;
    selection_start: number;
    selection_end: number;
    composing_range: InputContextRange | undefined;
}

export interface InputContextRange {
    start: number;
    end: number;
}

export interface InputModifiers {
    shift?: boolean;
    ctrl?: boolean;
    alt?: boolean;
    meta?: boolean;
}

export interface InspectStateOptions {
    show_node_ids: boolean;
}

export interface KeyEvent {
    key: Key;
    modifiers?: InputModifiers;
}

export interface ListItemNode {}

export interface NodeEntry {
    node: Node;
    parent?: NodeId;
    children?: NodeId[];
    modifiers?: Modifier[];
}

export interface OrderedListNode {}

export interface PageBreakNode {}

export interface PageRect {
    page_idx: number;
    rect: Rect;
}

export interface ParagraphNode {
    align?: TextAlign;
}

export interface Position {
    node_id: NodeId;
    offset: number;
    affinity?: Affinity;
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
    border_style?: TableBorderStyle;
    align?: TableAlign;
    proportion?: number;
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

export type Affinity = "downstream" | "upstream";

export type Axis = "horizontal" | "vertical";

export type BackendKind = "cpu" | "gpu";

export type BlockquoteVariant = "left_line" | "left_quote" | "message_sent" | "message_received";

export type Break = "line" | "paragraph" | "page";

export type CalloutVariant = "info" | "success" | "warning" | "danger";

export type ClipboardIntent = { type: "paste"; html: string | undefined; text: string } | { type: "cut" } | { type: "copy" };

export type CompositionIntent = { type: "update"; text: string; replace_length: number | undefined } | { type: "set_region"; start: number; end: number } | { type: "commit"; text: string } | { type: "commit_as_is" } | { type: "cancel" };

export type DeletionIntent = { type: "selection" } | { type: "move"; movement: Movement } | { type: "surrounding"; before: number; after: number } | { type: "surrounding_code_points"; before: number; after: number };

export type Direction = "forward" | "backward";

export type EditorEvent = { type: "state_changed"; fields: StateField[] } | { type: "render_invalidated" } | { type: "font_manifest_missing"; family: string; weight: number } | { type: "font_data_missing"; family: string; weight: number; required: FontData[]; prefetch: FontData[] } | { type: "cursor_exited_document_start" };

export type Effect = { load_font: { family: string; weight: number; codepoints: number[] } };

export type FontData = { type: "base" } | { type: "chunk"; index: number };

export type FormattingIntent = { type: "toggle_modifier"; modifier_type: ModifierType } | { type: "set_modifier"; modifier: Modifier } | { type: "clear_modifiers" };

export type HistoryIntent = { type: "undo" } | { type: "redo" };

export type HorizontalRuleVariant = "line" | "dashed_line" | "circle_line" | "diamond_line" | "circle" | "diamond" | "three_circles" | "three_diamonds" | "zigzag";

export type InsertionIntent = { type: "text"; text: string } | { type: "break"; kind: Break } | { type: "node"; node: Node };

export type Intent = { type: "insertion"; intent: InsertionIntent } | { type: "deletion"; intent: DeletionIntent } | { type: "formatting"; intent: FormattingIntent } | { type: "selection"; intent: SelectionIntent } | { type: "node"; intent: NodeIntent } | { type: "clipboard"; intent: ClipboardIntent } | { type: "composition"; intent: CompositionIntent } | { type: "navigation"; intent: NavigationIntent } | { type: "history"; intent: HistoryIntent };

export type Key = "enter" | "backspace" | "delete" | "tab" | "escape";

export type LayoutMode = { type: "paginated"; page_width: number; page_height: number; page_margin_top: number; page_margin_bottom: number; page_margin_left: number; page_margin_right: number } | { type: "continuous"; max_width: number };

export type Message = { type: "key"; event: KeyEvent } | { type: "pointer"; event: PointerEvent } | { type: "intent"; intent: Intent } | { type: "system"; event: SystemEvent };

export type Modifier = { type: "bold" } | { type: "italic" } | { type: "underline" } | { type: "strikethrough" } | { type: "font_size"; value: number } | { type: "font_family"; value: string } | { type: "font_weight"; value: number } | { type: "text_color"; value: string } | { type: "background_color"; value: string } | { type: "letter_spacing"; value: number } | { type: "link"; href: string } | { type: "ruby"; text: string } | { type: "line_height"; value: number } | { type: "block_gap"; value: number } | { type: "paragraph_indent"; value: number };

export type Movement = { type: "grapheme"; direction: Direction } | { type: "word"; direction: Direction } | { type: "sentence"; direction: Direction } | { type: "line"; direction: Direction; axis: Axis } | { type: "block"; direction: Direction } | { type: "page"; direction: Direction } | { type: "document"; direction: Direction };

export type NavigationIntent = { type: "move"; movement: Movement; extend: boolean };

export type Node = ({ type: "root" } & RootNode) | ({ type: "paragraph" } & ParagraphNode) | ({ type: "blockquote" } & BlockquoteNode) | ({ type: "callout" } & CalloutNode) | ({ type: "text" } & TextNode) | ({ type: "bullet_list" } & BulletListNode) | ({ type: "ordered_list" } & OrderedListNode) | ({ type: "list_item" } & ListItemNode) | ({ type: "fold" } & FoldNode) | ({ type: "fold_title" } & FoldTitleNode) | ({ type: "fold_content" } & FoldContentNode) | ({ type: "table" } & TableNode) | ({ type: "table_row" } & TableRowNode) | ({ type: "table_cell" } & TableCellNode) | ({ type: "image" } & ImageNode) | ({ type: "file" } & FileNode) | ({ type: "embed" } & EmbedNode) | ({ type: "archived" } & ArchivedNode) | ({ type: "hard_break" } & HardBreakNode) | ({ type: "horizontal_rule" } & HorizontalRuleNode) | ({ type: "page_break" } & PageBreakNode);

export type NodeId = string;

export type NodeIntent = { type: "delete"; id: NodeId } | { type: "set_attrs"; id: NodeId; attrs: Node } | { type: "toggle_fold"; id: NodeId } | { type: "table"; id: NodeId; op: TableOp };

export type PointerEvent = { type: "down"; page: number; x: number; y: number; count: number; modifiers?: InputModifiers };

export type SelectionIntent = { type: "all" } | { type: "set"; selection: Selection } | { type: "set_flat"; start: number; end: number };

export type StateField = "doc" | "selection" | "cursor" | "page_sizes" | "modifiers";

export type SystemEvent = { type: "initialize" } | { type: "resize"; width: number; height: number; scale_factor: number } | { type: "set_focused"; focused: boolean } | { type: "font_manifest_loaded"; family: string; weight: number } | { type: "font_base_loaded"; family: string; weight: number } | { type: "font_chunk_loaded"; family: string; weight: number } | { type: "set_external_height"; node_id: NodeId; height: number };

export type TableAlign = "left" | "center" | "right";

export type TableBorderStyle = "solid" | "dashed" | "dotted" | "none";

export type TableOp = { type: "insert_axis"; axis: Axis; index: number; before: boolean } | { type: "delete_axis"; axis: Axis; index: number } | { type: "move_axis"; axis: Axis; from: number; to: number } | { type: "select_axis"; axis: Axis | undefined } | { type: "set_column_widths"; widths: number[] };

export type TextAlign = "left" | "center" | "right" | "justify";


declare class Editor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cursor(): PageRect | undefined;
    enqueue(message: Message): void;
    input_context(before_limit: number, after_limit: number): InputContext;
    inspect_state(options?: InspectStateOptions | null): string;
    inspect_state_as_macro(): string;
    page_sizes(): Size[];
    render_page_to_buffer(page: number, width: number, height: number): Uint8Array;
    selection(): Selection;
    tick(): EditorEvent[];
}

declare class EditorHost {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    build_fallback_font_manifests(entries: any): Uint8Array;
    build_font_manifest(chunk_codepoints: any): Uint8Array;
    static create(kind?: BackendKind | null): Promise<EditorHost>;
    create_editor(doc: Doc, selection: Selection, viewport: Viewport): Editor;
    encode_font(ttf_data: Uint8Array, chunk_codepoints: any): EncodedFont;
    get_font_codepoints(ttf_data: Uint8Array): any;
    get_font_metadata(data: Uint8Array): FontMetadata;
    load_fallback_font_manifests(data: Uint8Array): void;
    load_font_base(family: string, weight: number, data: Uint8Array): void;
    load_font_chunk(family: string, weight: number, data: Uint8Array): void;
    load_font_manifest(family: string, weight: number, data: Uint8Array): void;
    load_icu_data(data: Uint8Array): void;
    set_font_families(families: FontFamily[]): void;
    set_phantom_font_families(families: string[]): void;
}

export type { Editor, EditorHost };

export function createInstance(wasmModule: WebAssembly.Module): Promise<{
    Editor: typeof Editor;
    EditorHost: typeof EditorHost;
}>;
