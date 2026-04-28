/* tslint:disable */
/* eslint-disable */
/**
 * A document position: the triple `(node_id, offset, affinity)`.
 *
 * `Position` is a plain value type (POD) with no automatic validation.
 * Its invariants are documented below; violating positions either
 * resolve to `None` via [`Position::resolve`] (value-level invariants)
 * or produce incorrect behavior in downstream code (structural
 * invariants).
 *
 * # Invariants
 *
 * - `node_id` must refer to a **text node** or a **container node**
 *   (a node whose schema allows children). Non-text leaf nodes
 *   (e.g. `hard_break`, `horizontal_rule`, `image`, `page_break`,
 *   `embed`, `file`) must **never** appear as `node_id`; such
 *   locations are represented by the parent container's boundary
 *   (the offset between the siblings of the leaf).
 *   *Not currently enforced.*
 *
 * - `offset` must lie within the node's valid range:
 *   - Text node: `0..=char_count` (unicode codepoint units, **not** bytes).
 *   - Container node: `0..=children.len()`.
 *   *Not currently enforced.*
 *
 * # Semantics of `offset`
 *
 * `offset` names the **boundary between** elements, not an element itself.
 *
 * - In a **text node**, `offset` is a unicode codepoint index between
 *   chars. For `"hello"`, offset `0` is before `'h'`, offset `5` is
 *   after `'o'`.
 * - In a **container node**, `offset` is an index between children.
 *   For `blockquote { p1, p2, p3 }`, `offset: 1` names the boundary
 *   **between `p1` and `p2`** — it does NOT point at `p2` itself.
 *   - Empty container cursor: `offset = 0`.
 *   - End of container: `offset = children.len()` (e.g. 3 in the
 *     example above — the position after `p3`).
 *
 * # Semantics of `affinity`
 *
 * See [`Affinity`].
 */
export interface Position {
    node_id: NodeId;
    offset: number;
    affinity?: Affinity;
}

/**
 * A document selection: an ordered pair of positions with directional intent.
 *
 * `Selection` is a plain value type (POD) with no automatic validation.
 * Structural invariants (subtree constraint, affinity mutual
 * exclusion, affinity agreement) are the responsibility of
 * command/transaction implementations; constructors do **not**
 * enforce them.
 *
 * # `anchor` vs `head`
 *
 * - `anchor`: the fixed endpoint of the selection. It stays in place
 *   under range-extension operations (shift+arrow, shift+click, etc.).
 * - `head`: the moving endpoint — the caret.
 *
 * Direction is **preserved, never normalized**. A selection where
 * `anchor` sorts after `head` (a backward selection) is a distinct,
 * valid state from its forward counterpart. The two differ in which
 * endpoint future range extensions will move, so normalizing would
 * lose user intent.
 *
 * # Invariants
 *
 * - **Subtree constraint**: `anchor` and `head` must not lie in each
 *   other's subtrees. A selection that starts outside a nested node
 *   and ends inside it (or vice versa) is not representable.
 *   *Upheld by command/transaction implementations; constructors do
 *   not enforce this.*
 *
 * - **Affinity mutual exclusion (non-collapsed)**: when
 *   `anchor != head`, `anchor.affinity` points toward `head` and
 *   `head.affinity` points toward `anchor`.
 *   *Upheld by command/transaction implementations.*
 *
 * - **Affinity agreement (collapsed)**: when `anchor == head` (all
 *   three fields of `Position` match), the two affinities are equal.
 *   A caret has a single direction; the specific value (Up/Down) is
 *   free.
 *   *Upheld by command/transaction implementations.*
 *
 * # Node selection
 *
 * Selecting a non-text node (e.g. clicking an image) is represented
 * the same way as selecting a range of text: by two positions that
 * bracket the target. For `root { paragraph, image, paragraph }`,
 * selecting the image forward produces
 *
 * ```text
 * Selection {
 *     anchor: Position { node: root, offset: 1, affinity: Downstream },
 *     head:   Position { node: root, offset: 2, affinity: Upstream },
 * }
 * ```
 *
 * The backward form — `anchor` at offset 2 `Upstream`, `head` at
 * offset 1 `Downstream` — is a distinct valid state representing the
 * same visual selection with the opposite user intent.
 */
export interface Selection {
    anchor: Position;
    head: Position;
}

/**
 * An IME composition range, expressed in flat-offset coordinates.
 *
 * `start` and `end` are **flat offsets** — absolute positions over the
 * entire document, not per-node offsets. Flat offsets are defined by
 * the flat-offset scheme implemented in this crate's `flat` module
 * (see `FlatClass`, `ResolvedPositionFlatExt`).
 *
 * A composition can span multiple nodes. The set of nodes covered by
 * a composition is computed on demand by walking the document from
 * the flat range; `Composition` itself stores no node identity and
 * no caching.
 */
export interface Composition {
    start: number;
    end: number;
}

/**
 * The directional bias of a [`Position`](crate::Position) at a boundary.
 *
 * Affinity disambiguates which side of a boundary a position belongs to.
 * Its meaning depends on the kind of node that contains the position:
 *
 * - **Text node**: determines whether a position between two characters
 *   leans toward the preceding char or the following char. Primarily used
 *   at soft-wrap boundaries to decide whether a caret is shown at the end
 *   of the upper line or at the start of the lower line. The role may
 *   be extended to other situations in the future.
 * - **Container node**: when a boundary position must be resolved to a
 *   single child node, affinity picks between the preceding and the
 *   following child. `Upstream` → `child[offset - 1]` (preceding);
 *   `Downstream` → `child[offset]` (following).
 */
export type Affinity = "downstream" | "upstream";

/**
 * chunk별 flat 정수 배열 `[start0, end0, start1, end1, ...]` (inclusive).
 */
export interface FontWeight {
    value: number;
    hash: string;
    chunks: number[][];
}

/**
 *Auto-generated discriminant enum variants
 */
export type ModifierType = "bold" | "italic" | "underline" | "strikethrough" | "font_size" | "font_family" | "font_weight" | "text_color" | "background_color" | "letter_spacing" | "link" | "ruby" | "line_height" | "block_gap" | "paragraph_indent" | "alignment";

/**
 *Auto-generated discriminant enum variants
 */
export type NodeType = "root" | "paragraph" | "blockquote" | "callout" | "text" | "bullet_list" | "ordered_list" | "list_item" | "fold" | "fold_title" | "fold_content" | "table" | "table_row" | "table_cell" | "image" | "file" | "embed" | "archived" | "hard_break" | "horizontal_rule" | "page_break";

export interface AlignmentValue {
    value: Alignment;
}

export interface ArchivedNode {
    id: string | undefined;
}

export interface BackgroundColorValue {
    value: string;
}

export interface Block {
    id: NodeId;
    node: Node;
}

export interface BlockGapValue {
    value: number;
}

export interface BlockState {
    ancestors: Block[];
    nodes: Block[];
}

export interface BlockquoteNode {
    variant?: BlockquoteVariant;
}

export interface BuiltFont {
    hash: string;
    /**
     * chunk별 flat 페어 `[start0, end0, start1, end1, ...]` (inclusive).
     */
    coverage: number[][];
    base: Uint8Array;
    chunks: Uint8Array[];
}

export interface BulletListNode {}

export interface CalloutNode {
    variant?: CalloutVariant;
}

export interface CursorMetrics {
    page_idx: number;
    caret: Rect;
    line: Rect;
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

export interface FileNode {
    id: string | undefined;
}

export interface FoldContentNode {}

export interface FoldNode {}

export interface FoldTitleNode {}

export interface FontFamily {
    name: string;
    source: FontFamilySource;
    weights: FontWeight[];
}

export interface FontFamilyValue {
    value: string;
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

export interface FontSizeValue {
    value: number;
}

export interface FontWeightValue {
    value: number;
}

export interface Fragment {
    node: Node;
    modifiers?: Modifier[];
    children?: Fragment[];
}

export interface HardBreakNode {}

export interface HorizontalRuleNode {
    variant?: HorizontalRuleVariant;
}

export interface ImageNode {
    id: string | undefined;
    proportion?: number;
}

export interface Ime {
    text: string;
    window_start: number;
    selection: ImeRange;
    composing: ImeRange | undefined;
}

export interface ImeRange {
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

export interface LetterSpacingValue {
    value: number;
}

export interface LineHeightValue {
    value: number;
}

export interface LinkValue {
    href: string;
}

export interface ListItemNode {}

export interface ModifierState {
    bold: Tri<undefined>;
    italic: Tri<undefined>;
    underline: Tri<undefined>;
    strikethrough: Tri<undefined>;
    font_size: Tri<FontSizeValue>;
    font_family: Tri<FontFamilyValue>;
    font_weight: Tri<FontWeightValue>;
    text_color: Tri<TextColorValue>;
    background_color: Tri<BackgroundColorValue>;
    letter_spacing: Tri<LetterSpacingValue>;
    link: Tri<LinkValue>;
    ruby: Tri<RubyValue>;
    line_height: Tri<LineHeightValue>;
    block_gap: Tri<BlockGapValue>;
    paragraph_indent: Tri<ParagraphIndentValue>;
    alignment: Tri<AlignmentValue>;
}

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

export interface ParagraphIndentValue {
    value: number;
}

export interface ParagraphNode {}

export interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface RootNode {}

export interface RubyValue {
    text: string;
}

export interface Size {
    width: number;
    height: number;
}

export interface Subtree {
    id: NodeId;
    node: Node;
    modifiers: Modifier[];
    children: Subtree[];
}

export interface TableCellNode {
    col_width: number | undefined;
}

export interface TableNode {
    border_style?: TableBorderStyle;
    proportion?: number;
}

export interface TableRowNode {}

export interface TextColorValue {
    value: string;
}

export interface TextNode {
    text: string;
}

export interface TransactionMeta {
    history: HistoryMeta;
}

export interface Viewport {
    width: number;
    height: number;
    scale_factor: number;
}

export type Alignment = "left" | "center" | "right" | "justify";

export type Axis = "horizontal" | "vertical";

export type BlockquoteVariant = "left_line" | "left_quote" | "message_sent" | "message_received";

export type Break = "line" | "paragraph" | "page";

export type CalloutVariant = "info" | "success" | "warning" | "danger";

export type ClipboardOp = { type: "paste"; html: string | undefined; text: string } | { type: "cut" } | { type: "copy" };

export type CompositionOp = { type: "update"; text: string; replace_length: number | undefined } | { type: "set_region"; start: number; end: number } | { type: "commit"; text: string } | { type: "commit_as_is" } | { type: "cancel" } | { type: "flat"; ops: FlatImeOp[] };

export type DeletionOp = { type: "selection" } | { type: "move"; movement: Movement } | { type: "surrounding"; before: number; after: number } | { type: "surrounding_code_points"; before: number; after: number };

export type Direction = "forward" | "backward";

export type DocOp = { type: "set_attrs"; attrs: DocumentAttrs };

export type EditorEvent = { type: "state_changed"; fields: StateField[] } | { type: "render_invalidated" } | { type: "font_data_missing"; family: string; weight: number; required: FontData[]; prefetch: FontData[] } | { type: "cursor_exited_document_start" } | { type: "transaction_committed"; steps: Step[]; meta: TransactionMeta };

export type Effect = { load_font: { family: string; weight: number; codepoints: number[] } };

export type FlatImeOp = { type: "set_selection"; start: number; end: number } | { type: "replace_selection"; text: string } | { type: "compose"; text: string } | { type: "delete_surrounding"; before: number; after: number } | { type: "delete_surrounding_utf16"; before: number; after: number } | { type: "set_composition"; start: number; end: number } | { type: "clear_composition" } | { type: "move_cursor"; delta: number };

export type FontData = { type: "base" } | { type: "chunk"; id: number };

export type FontFamilySource = "DEFAULT" | "USER" | "FALLBACK";

export type HistoryMeta = { type: "record" } | { type: "tagged"; tag: HistoryTag } | { type: "skip" };

export type HistoryOp = { type: "undo" } | { type: "redo" };

export type HistoryTag = { type: "auto_replacement" } | { type: "paste_html"; plain_text: string };

export type HorizontalRuleVariant = "line" | "dashed_line" | "circle_line" | "diamond_line" | "circle" | "diamond" | "three_circles" | "three_diamonds" | "zigzag";

export type InsertionOp = { type: "text"; text: string } | { type: "break"; kind: Break } | { type: "fragment"; fragment: Fragment };

export type Key = "enter" | "backspace" | "delete" | "tab" | "escape";

export type LayoutMode = { type: "paginated"; page_width: number; page_height: number; page_margin_top: number; page_margin_bottom: number; page_margin_left: number; page_margin_right: number } | { type: "continuous"; max_width: number };

export type Message = { type: "key"; event: KeyEvent } | { type: "pointer"; event: PointerEvent } | { type: "insertion"; op: InsertionOp } | { type: "deletion"; op: DeletionOp } | { type: "selection"; op: SelectionOp } | { type: "modifier"; op: ModifierOp } | { type: "doc"; op: DocOp } | { type: "node"; op: NodeOp } | { type: "clipboard"; op: ClipboardOp } | { type: "composition"; op: CompositionOp } | { type: "navigation"; op: NavigationOp } | { type: "history"; op: HistoryOp } | { type: "system"; event: SystemEvent };

export type Modifier = { type: "bold" } | { type: "italic" } | { type: "underline" } | { type: "strikethrough" } | { type: "font_size"; value: number } | { type: "font_family"; value: string } | { type: "font_weight"; value: number } | { type: "text_color"; value: string } | { type: "background_color"; value: string } | { type: "letter_spacing"; value: number } | { type: "link"; href: string } | { type: "ruby"; text: string } | { type: "line_height"; value: number } | { type: "block_gap"; value: number } | { type: "paragraph_indent"; value: number } | { type: "alignment"; value: Alignment };

export type ModifierOp = { type: "toggle"; modifier_type: ModifierType } | { type: "set"; modifier: Modifier } | { type: "clear_all" };

export type Movement = { type: "grapheme"; direction: Direction } | { type: "word"; direction: Direction } | { type: "sentence"; direction: Direction } | { type: "line"; direction: Direction; axis: Axis } | { type: "block"; direction: Direction } | { type: "page"; direction: Direction } | { type: "document"; direction: Direction };

export type NavigationOp = { type: "move"; movement: Movement; extend: boolean };

export type Node = ({ type: "root" } & RootNode) | ({ type: "paragraph" } & ParagraphNode) | ({ type: "blockquote" } & BlockquoteNode) | ({ type: "callout" } & CalloutNode) | ({ type: "text" } & TextNode) | ({ type: "bullet_list" } & BulletListNode) | ({ type: "ordered_list" } & OrderedListNode) | ({ type: "list_item" } & ListItemNode) | ({ type: "fold" } & FoldNode) | ({ type: "fold_title" } & FoldTitleNode) | ({ type: "fold_content" } & FoldContentNode) | ({ type: "table" } & TableNode) | ({ type: "table_row" } & TableRowNode) | ({ type: "table_cell" } & TableCellNode) | ({ type: "image" } & ImageNode) | ({ type: "file" } & FileNode) | ({ type: "embed" } & EmbedNode) | ({ type: "archived" } & ArchivedNode) | ({ type: "hard_break" } & HardBreakNode) | ({ type: "horizontal_rule" } & HorizontalRuleNode) | ({ type: "page_break" } & PageBreakNode);

export type NodeId = string;

export type NodeOp = { type: "delete"; id: NodeId } | { type: "set_attrs"; id: NodeId; attrs: Node } | { type: "table"; id: NodeId; op: TableOp };

export type PendingModifier = { type: "set"; modifier: Modifier } | { type: "unset"; ty: ModifierType };

export type PendingModifiers = PendingModifier[];

export type PointerEvent = { type: "down"; page: number; x: number; y: number; count: number; modifiers?: InputModifiers } | { type: "move"; page: number; x: number; y: number } | { type: "up" };

export type SelectionOp = { type: "all" } | { type: "set"; selection: Selection } | { type: "set_flat"; start: number; end: number };

export type StateField = "doc" | "doc_attrs" | "selection" | "cursor" | "page_sizes" | "ime" | "modifiers" | "block";

export type Step = { type: "insert_text"; node_id: NodeId; offset: number; text: string } | { type: "remove_text"; node_id: NodeId; offset: number; text: string } | { type: "insert_subtree"; parent_id: NodeId; index: number; subtree: Subtree } | { type: "remove_subtree"; parent_id: NodeId; index: number; subtree: Subtree } | { type: "move_node"; node_id: NodeId; old_parent: NodeId; old_index: number; new_parent: NodeId; new_index: number } | { type: "split_node"; node_id: NodeId; offset: number; new_node_id: NodeId } | { type: "merge_node"; node_id: NodeId; target_id: NodeId; offset: number } | { type: "set_node"; node_id: NodeId; old_node: Node; new_node: Node } | { type: "add_modifier"; node_id: NodeId; modifier: Modifier } | { type: "remove_modifier"; node_id: NodeId; modifier: Modifier } | { type: "set_selection"; old: Selection; new: Selection } | { type: "set_pending_modifiers"; old: PendingModifiers; new: PendingModifiers } | { type: "set_modifiers"; node_id: NodeId; old_modifiers: Modifier[]; new_modifiers: Modifier[] } | { type: "set_composition"; old: Composition | undefined; new: Composition | undefined } | { type: "set_document_attrs"; old: DocumentAttrs; new: DocumentAttrs };

export type SystemEvent = { type: "initialize" } | { type: "resize"; width: number; height: number; scale_factor: number } | { type: "set_focused"; focused: boolean } | { type: "font_base_loaded"; family: string; weight: number } | { type: "font_chunk_loaded"; family: string; weight: number; chunk_id: number } | { type: "set_external_height"; node_id: NodeId; height: number } | { type: "fonts_changed" };

export type TableBorderStyle = "solid" | "dashed" | "dotted" | "none";

export type TableOp = { type: "insert_axis"; axis: Axis; index: number; before: boolean } | { type: "delete_axis"; axis: Axis; index: number } | { type: "move_axis"; axis: Axis; from: number; to: number } | { type: "select_axis"; axis: Axis | undefined } | { type: "set_column_widths"; widths: number[] };

export type Tri<T> = { type: "absent" } | { type: "uniform"; value: T } | { type: "mixed" };


declare class Editor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    block_state(): BlockState;
    cursor(): CursorMetrics | undefined;
    document_attrs(): DocumentAttrs;
    enqueue(message: Message): void;
    ime(before_limit: number, after_limit: number): Ime;
    inspect_state(options?: InspectStateOptions | null): string;
    inspect_state_as_macro(): string;
    modifier_state(): ModifierState;
    page_sizes(): Size[];
    render_page_to_buffer(page: number, width: number, height: number): Uint8Array;
    selection(): Selection;
    tick(): EditorEvent[];
}

declare class EditorHost {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    add_font_base(family: string, weight: number, data: Uint8Array): void;
    add_font_chunk(family: string, weight: number, chunk_id: number, data: Uint8Array): void;
    build_font(ttf_data: Uint8Array, chunk_codepoints: any): BuiltFont;
    static create(icu_data: Uint8Array): EditorHost;
    create_editor(doc: Doc, selection: Selection, viewport: Viewport): Editor;
    get_font_codepoints(ttf_data: Uint8Array): any;
    get_font_metadata(data: Uint8Array): FontMetadata;
    set_fonts(families: FontFamily[]): void;
}

export type { Editor, EditorHost };

export function createInstance(wasmModule: WebAssembly.Module): Promise<{
    Editor: typeof Editor;
    EditorHost: typeof EditorHost;
}>;
