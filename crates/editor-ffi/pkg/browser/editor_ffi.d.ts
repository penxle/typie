/* tslint:disable */
/* eslint-disable */
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
 * Atomic application unit. Wire / db row / FFI boundary.
 *
 * `ops` must be in parents-before-children topological order — sender's
 * responsibility. The receiver (`OpGraph::receive_changeset` Phase A)
 * rejects out-of-order ops via the per-op parents-known check. Standard
 * sender APIs (`OpGraph::topo_sort`, `OpGraph::missing_changesets_for`,
 * sequential `OpGraph::add` followed by `OpGraph::commit`) satisfy this
 * naturally.
 */
export interface Changeset<P> {
    ops: Op<P>[];
}

/**
 * One node in the op-DAG. `id` is the op's unique identifier (also reused as
 * the semantic identifier — RGA element id, OR-Set add token — by the
 * payload). `parents` are the op-DAG parents of this op (the heads of the
 * store at the moment this op was created). Stored normalized: sorted
 * ascending, no duplicates.
 */
export interface Op<P> {
    id: Dot;
    parents: Dot[];
    payload: P;
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
 * Which side of a CRDT element the cursor sits on.
 */
export type Bind = "left" | "right";

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
export type NodeType = "root" | "paragraph" | "blockquote" | "callout" | "text" | "bullet_list" | "ordered_list" | "list_item" | "fold" | "fold_title" | "fold_content" | "table" | "table_row" | "table_cell" | "image" | "file" | "embed" | "archived" | "hard_break" | "horizontal_rule" | "page_break" | "tab";

export interface AlignmentValue {
    value: Alignment;
}

export interface BackgroundColorValue {
    value: string;
}

export interface Block {
    id: Dot;
    node: PlainNode;
}

export interface BlockGapValue {
    value: number;
}

export interface BlockState {
    ancestors: Block[];
    nodes: Block[];
}

export interface ChangesetEntry {
    id: string;
    bytes: Uint8Array;
}

export interface CharacterCounts {
    doc_with_whitespace: number;
    doc_without_whitespace: number;
    doc_without_whitespace_and_punctuation: number;
    selection_with_whitespace: number;
    selection_without_whitespace: number;
    selection_without_whitespace_and_punctuation: number;
}

export interface ClipboardPayload {
    html: string;
    text: string;
}

export interface CursorMetrics {
    page_idx: number;
    caret: Rect;
    line: Rect;
}

export interface DecorationStyle {
    background: string | undefined;
    background_radius?: number | undefined;
    background_inset?: number | undefined;
    underline: Underline | undefined;
}

export interface ExternalElement {
    page_idx: number;
    node: Dot;
    bounds: Rect;
    is_selected: boolean;
    data: ExternalElementData;
}

export interface FontFamily {
    name: string;
    source: FontFamilySource;
    weights: FontWeight[];
}

export interface FontFamilyValue {
    value: string;
}

export interface FontSizeValue {
    value: number;
}

export interface FontWeightValue {
    value: number;
}

export interface Fragment {
    node: PlainNode;
    modifiers?: Modifier[];
    children?: Fragment[];
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

export interface LinkRect {
    page_idx: number;
    href: string;
    rects: Rect[];
}

export interface LinkValue {
    href: string;
}

export interface Marker {
    modifiers: Modifier[];
}

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
    effective_bold: Tri<undefined>;
}

export interface PageRect {
    page_idx: number;
    rect: Rect;
}

export interface ParagraphIndentValue {
    value: number;
}

export interface PartitionedChangesets {
    ready: Uint8Array;
    blocked: Uint8Array;
}

export interface PlaceholderMetrics {
    page_idx: number;
    rect: Rect;
    font_size: number | undefined;
    line_height: number | undefined;
    letter_spacing: number | undefined;
    align: Alignment | undefined;
}

export interface PlainArchivedNode {
    id: string | undefined;
}

export interface PlainBlockquoteNode {
    variant?: BlockquoteVariant;
}

export interface PlainBulletListNode {}

export interface PlainCalloutNode {
    variant?: CalloutVariant;
}

export interface PlainDoc {
    root: PlainNodeEntry;
}

export interface PlainEmbedNode {
    id: string | undefined;
}

export interface PlainFileNode {
    id: string | undefined;
}

export interface PlainFoldContentNode {}

export interface PlainFoldNode {}

export interface PlainFoldTitleNode {}

export interface PlainHardBreakNode {}

export interface PlainHorizontalRuleNode {
    variant?: HorizontalRuleVariant;
}

export interface PlainImageNode {
    id: string | undefined;
    proportion?: number;
}

export interface PlainListItemNode {}

export interface PlainNodeEntry {
    node: PlainNode;
    modifiers: Record<ModifierType, Modifier>;
    marker?: Marker | undefined;
    children: PlainNodeEntry[];
}

export interface PlainOrderedListNode {}

export interface PlainPageBreakNode {}

export interface PlainParagraphNode {}

export interface PlainRootNode {
    layout_mode: LayoutMode;
}

export interface PlainTabNode {}

export interface PlainTableCellNode {
    col_width: number | undefined;
    background_color: string | undefined;
}

export interface PlainTableNode {
    border_style?: TableBorderStyle;
    proportion?: number;
}

export interface PlainTableRowNode {}

export interface PlainTextNode {
    text: string;
}

export interface Position {
    node: Dot;
    offset: number;
    affinity: Affinity;
}

export interface RawTextReplacementRule {
    id: string;
    matchPattern: string;
    substitute: string;
    regex: boolean;
}

export interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface RubyValue {
    text: string;
}

export interface SearchOptions {
    match_whole_word?: boolean;
}

export interface Selection {
    anchor: Position;
    head: Position;
}

export interface SelectionEndpoints {
    from: PageRect;
    to: PageRect;
    from_position: Position;
    to_position: Position;
}

export interface Size {
    width: number;
    height: number;
}

export interface StablePosition {
    chain: Dot[];
    binding: StablePositionBinding;
    affinity: Affinity;
}

export interface StableSelection {
    anchor: StablePosition;
    head: StablePosition;
}

export interface TableOverlay {
    table_id: Dot;
    page_idx: number;
    bounds: Rect;
    border_style: TableBorderStyle;
    align: Alignment;
    proportion: number;
    content_width: number;
    rows: TableOverlayRow[];
    columns: TableOverlayColumn[];
    row_count: number;
    is_last_row_fragment: boolean;
    is_focused: boolean;
    focused_row_index: number | undefined;
    focused_col_index: number | undefined;
    is_cell_selection: boolean;
    cell_selection_background_color: string | undefined;
    cell_selection_row_start: number | undefined;
    cell_selection_row_end: number | undefined;
    cell_selection_col_start: number | undefined;
    cell_selection_col_end: number | undefined;
}

export interface TableOverlayColumn {
    index: number;
    width_as_px: number;
    position: number;
    background_color: string | undefined;
}

export interface TableOverlayRow {
    index: number;
    height: number;
    position: number;
    background_color: string | undefined;
}

export interface TextColorValue {
    value: string;
}

export interface TrackedRange {
    id: string;
    group: string;
    anchor: Position;
    head: Position;
    metadata: string;
    rects: PageRect[];
    text: string;
}

export interface TrackedRangeEndpoints {
    id: string;
    group: string;
    anchor: Position;
    head: Position;
}

export interface TrackedRangeHit {
    id: string;
    group: string;
    rects: PageRect[];
}

export interface TransactionMeta {
    history: HistoryMeta;
}

export interface Underline {
    color: string;
    style: UnderlineStyle;
    thickness: number;
}

export interface Viewport {
    width: number;
    height: number;
    scale_factor: number;
}

export type Alignment = "left" | "center" | "right" | "justify";

export type ArchivedNodeAttr = { type: "id" } & string | undefined;

export type Axis = "horizontal" | "vertical";

export type BlockquoteNodeAttr = { type: "variant" } & BlockquoteVariant;

export type BlockquoteVariant = "left_line" | "left_quote" | "message_sent" | "message_received";

export type Break = "line" | "paragraph" | "page";

export type BulletListNodeAttr = void;

export type CalloutNodeAttr = { type: "variant" } & CalloutVariant;

export type CalloutVariant = "info" | "success" | "warning" | "danger";

export type ClipboardOp = { type: "paste"; html: string | undefined; text: string } | { type: "repaste_as_text" } | { type: "cut" };

export type DeletionOp = { type: "selection" } | { type: "move"; movement: Movement } | { type: "surrounding"; before: number; after: number } | { type: "surrounding_code_points"; before: number; after: number };

export type Direction = "forward" | "backward";

export type DndDropPayload = { type: "internal_selection" } | { type: "text"; text: string; html: string | undefined } | { type: "files"; image_count: number; file_count: number };

export type DndOp = { type: "start_internal_selection" } | { type: "enter_external"; payload: ExternalDndPayloadKind } | { type: "over"; page: number; x: number; y: number; modifiers?: InputModifiers } | { type: "leave" } | { type: "drop"; page: number; x: number; y: number; payload: DndDropPayload; modifiers?: InputModifiers } | { type: "end" };

export type Dot = string;

export type EditorEvent = { type: "state_changed"; fields: StateField[] } | { type: "render_invalidated" } | { type: "font_data_missing"; family: string; weight: number; required: FontData[]; prefetch: FontData[] } | { type: "cursor_exited_document_start" } | { type: "tracked_range_replace_result"; id: string; outcome: TrackedRangeReplaceOutcome };

export type Effect = { load_font: { family: string; weight: number; codepoints: number[] } };

export type EmbedNodeAttr = { type: "id" } & string | undefined;

export type ExternalDndPayloadKind = "text" | "html" | "image_files" | "files" | "mixed_files";

export type ExternalElementData = { type: "image"; id: string | undefined; proportion: number } | { type: "file"; id: string | undefined } | { type: "embed"; id: string | undefined } | { type: "archived"; id: string | undefined };

export type FileNodeAttr = { type: "id" } & string | undefined;

export type FlatImeOp = { type: "set_selection"; start: number; end: number } | { type: "replace_selection"; text: string } | { type: "compose"; text: string } | { type: "delete_surrounding"; before: number; after: number } | { type: "delete_surrounding_utf16"; before: number; after: number } | { type: "set_composition"; start: number; end: number } | { type: "clear_composition" } | { type: "commit_as_is" } | { type: "move_cursor"; delta: number };

export type FoldContentNodeAttr = void;

export type FoldNodeAttr = void;

export type FoldTitleNodeAttr = void;

export type FontData = { type: "base" } | { type: "chunk"; id: number };

export type FontFamilySource = "DEFAULT" | "USER" | "FALLBACK";

export type HardBreakNodeAttr = void;

export type HistoryMeta = { type: "record" } | { type: "tagged"; tag: HistoryTag } | { type: "skip" };

export type HistoryOp = { type: "undo" } | { type: "redo" };

export type HistoryTag = { type: "auto_replacement" } | { type: "paste_html"; plain_text: string; start: number | undefined };

export type HorizontalRuleNodeAttr = { type: "variant" } & HorizontalRuleVariant;

export type HorizontalRuleVariant = "line" | "dashed_line" | "circle_line" | "diamond_line" | "circle" | "diamond" | "three_circles" | "three_diamonds" | "zigzag";

export type ImageNodeAttr = ({ type: "id" } & string | undefined) | ({ type: "proportion" } & number);

export type InsertionOp = { type: "text"; text: string } | { type: "break"; kind: Break } | { type: "fragment"; fragment: Fragment };

export type InteractiveHit = { type: "fold_title"; id: Dot; text_rect: Rect | undefined } | { type: "callout_icon"; id: Dot; next_variant: CalloutVariant };

export type Key = "enter" | "backspace" | "delete" | "tab" | "escape";

export type LayoutMode = { type: "paginated"; page_width: number; page_height: number; page_margin_top: number; page_margin_bottom: number; page_margin_left: number; page_margin_right: number } | { type: "continuous"; max_width: number };

export type ListItemNodeAttr = void;

export type Message = { type: "key"; event: KeyEvent } | { type: "insertion"; op: InsertionOp } | { type: "deletion"; op: DeletionOp } | { type: "selection"; op: SelectionOp } | { type: "modifier"; op: ModifierOp } | { type: "node"; op: NodeOp } | { type: "view"; op: ViewOp } | { type: "clipboard"; op: ClipboardOp } | { type: "text_input"; ops: FlatImeOp[] } | { type: "dnd"; op: DndOp } | { type: "navigation"; op: NavigationOp } | { type: "history"; op: HistoryOp } | { type: "system"; event: SystemEvent } | { type: "tracked_range"; op: TrackedRangeOp };

export type Modifier = { type: "bold" } | { type: "italic" } | { type: "underline" } | { type: "strikethrough" } | { type: "font_size"; value: number } | { type: "font_family"; value: string } | { type: "font_weight"; value: number } | { type: "text_color"; value: string } | { type: "background_color"; value: string } | { type: "letter_spacing"; value: number } | { type: "link"; href: string } | { type: "ruby"; text: string } | { type: "line_height"; value: number } | { type: "block_gap"; value: number } | { type: "paragraph_indent"; value: number } | { type: "alignment"; value: Alignment };

export type ModifierOp = { type: "toggle"; modifier_type: ModifierType } | { type: "set"; modifier: Modifier } | { type: "set_on_node"; id: Dot; modifier: Modifier } | { type: "edit"; modifier_type: ModifierType; modifier: Modifier | undefined } | { type: "clear_all" };

export type Movement = { type: "grapheme"; direction: Direction } | { type: "word"; direction: Direction } | { type: "sentence"; direction: Direction } | { type: "line"; direction: Direction; axis: Axis } | { type: "page"; direction: Direction } | { type: "document"; direction: Direction };

export type NavigationOp = { type: "move"; movement: Movement; extend: boolean };

export type NodeAttr = { type: "root"; attr: RootNodeAttr } | { type: "paragraph"; attr: ParagraphNodeAttr } | { type: "blockquote"; attr: BlockquoteNodeAttr } | { type: "callout"; attr: CalloutNodeAttr } | { type: "text"; attr: TextNodeAttr } | { type: "bullet_list"; attr: BulletListNodeAttr } | { type: "ordered_list"; attr: OrderedListNodeAttr } | { type: "list_item"; attr: ListItemNodeAttr } | { type: "fold"; attr: FoldNodeAttr } | { type: "fold_title"; attr: FoldTitleNodeAttr } | { type: "fold_content"; attr: FoldContentNodeAttr } | { type: "table"; attr: TableNodeAttr } | { type: "table_row"; attr: TableRowNodeAttr } | { type: "table_cell"; attr: TableCellNodeAttr } | { type: "image"; attr: ImageNodeAttr } | { type: "file"; attr: FileNodeAttr } | { type: "embed"; attr: EmbedNodeAttr } | { type: "archived"; attr: ArchivedNodeAttr } | { type: "hard_break"; attr: HardBreakNodeAttr } | { type: "horizontal_rule"; attr: HorizontalRuleNodeAttr } | { type: "page_break"; attr: PageBreakNodeAttr } | { type: "tab"; attr: TabNodeAttr };

export type NodeOp = { type: "delete"; id: Dot } | { type: "set_attrs"; id: Dot; attrs: PlainNode } | { type: "table"; id: Dot; op: TableOp };

export type OrderedListNodeAttr = void;

export type PageBreakNodeAttr = void;

export type ParagraphNodeAttr = void;

export type PendingModifier = { type: "set"; modifier: Modifier } | { type: "unset"; ty: ModifierType };

export type PendingModifiers = PendingModifier[];

export type PlainNode = ({ type: "root" } & PlainRootNode) | ({ type: "paragraph" } & PlainParagraphNode) | ({ type: "blockquote" } & PlainBlockquoteNode) | ({ type: "callout" } & PlainCalloutNode) | ({ type: "text" } & PlainTextNode) | ({ type: "bullet_list" } & PlainBulletListNode) | ({ type: "ordered_list" } & PlainOrderedListNode) | ({ type: "list_item" } & PlainListItemNode) | ({ type: "fold" } & PlainFoldNode) | ({ type: "fold_title" } & PlainFoldTitleNode) | ({ type: "fold_content" } & PlainFoldContentNode) | ({ type: "table" } & PlainTableNode) | ({ type: "table_row" } & PlainTableRowNode) | ({ type: "table_cell" } & PlainTableCellNode) | ({ type: "image" } & PlainImageNode) | ({ type: "file" } & PlainFileNode) | ({ type: "embed" } & PlainEmbedNode) | ({ type: "archived" } & PlainArchivedNode) | ({ type: "hard_break" } & PlainHardBreakNode) | ({ type: "horizontal_rule" } & PlainHorizontalRuleNode) | ({ type: "page_break" } & PlainPageBreakNode) | ({ type: "tab" } & PlainTabNode);

export type PointerStyle = "default" | "text" | "pointer";

export type RootNodeAttr = { type: "layout_mode" } & LayoutMode;

export type SelectionExpansionUnit = "word" | "sentence" | "paragraph" | "all";

export type SelectionOp = { type: "set"; selection: Selection } | { type: "set_frozen"; selection: StableSelection } | { type: "unset" } | { type: "set_at"; page: number; x: number; y: number } | { type: "set_flat"; start: number; end: number } | { type: "extend_to"; anchor: Position; head_page: number; head_x: number; head_y: number; base_selection: Selection | undefined; allow_collapse?: boolean } | { type: "select_unit_at"; page: number; x: number; y: number; unit: SelectionPointUnit } | { type: "expand"; unit: SelectionExpansionUnit };

export type SelectionPointUnit = "word" | "sentence" | "paragraph";

export type StablePositionBinding = { type: "adjacent"; anchor: Dot; bind: Bind } | { type: "container_start" };

export type StateField = "doc" | "root_attrs" | "selection" | "cursor" | "page_sizes" | "external_elements" | "table_overlays" | "link_rects" | "ime" | "modifiers" | "block" | "tracked_ranges" | "last_history_tag" | "placeholder";

export type SystemEvent = { type: "initialize" } | { type: "resize"; width: number; height: number; scale_factor: number } | { type: "set_focused"; focused: boolean } | { type: "theme_variant_changed" } | { type: "font_base_loaded"; family: string; weight: number } | { type: "font_chunk_loaded"; family: string; weight: number; chunk_id: number } | { type: "set_external_height"; node_id: Dot; height: number } | { type: "fonts_changed" };

export type TabNodeAttr = void;

export type TableBorderStyle = "solid" | "dashed" | "dotted" | "none";

export type TableCellNodeAttr = ({ type: "col_width" } & number | undefined) | ({ type: "background_color" } & string | undefined);

export type TableNodeAttr = ({ type: "border_style" } & TableBorderStyle) | ({ type: "proportion" } & number);

export type TableOp = { type: "insert_axis"; axis: Axis; index: number; before: boolean } | { type: "delete_axis"; axis: Axis; index: number } | { type: "move_axis"; axis: Axis; from: number; to: number } | { type: "select_axis"; axis: Axis | undefined; index: number | undefined } | { type: "set_column_widths"; widths: number[] } | { type: "set_border_style"; border_style: TableBorderStyle } | { type: "set_proportion"; proportion: number } | { type: "set_axis_background_color"; axis: Axis; index: number; color: string | undefined } | { type: "set_cell_selection_background_color"; color: string | undefined };

export type TableRowNodeAttr = void;

export type TextNodeAttr = void;

export type ThemeVariant = "dark-black" | "dark-charcoal" | "dark-espresso" | "dark-graphite" | "dark-midnight" | "dark-navy" | "dark-obsidian" | "dark-storm" | "light-butter" | "light-latte" | "light-lavender" | "light-mint" | "light-peach" | "light-rose" | "light-snow" | "light-white";

export type TrackedRangeOp = { type: "add"; id: string; group: string; selection: Selection; metadata?: string } | { type: "add_frozen"; id: string; group: string; selection: StableSelection; metadata?: string } | { type: "remove"; id: string } | { type: "set_group"; id: string; group: string } | { type: "clear_group"; group: string } | { type: "invalidate"; id: string } | { type: "set_group_decoration"; group: string; style: DecorationStyle; enabled: boolean; z_index?: number } | { type: "remove_group_decoration"; group: string } | { type: "replace_text"; id: string; expected_text?: string | undefined; replacement: string };

export type TrackedRangeReplaceOutcome = "replaced" | "unknown_id" | "invalid" | "text_mismatch" | "invalid_replacement";

export type Tri<T> = { type: "absent" } | { type: "uniform"; value: T } | { type: "mixed" };

export type UnderlineStyle = "solid" | "dashed" | "wavy";

export type ViewOp = { type: "toggle_fold"; id: Dot };


declare class Editor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    attach_surface(page: number, handle: HTMLCanvasElement, width: number, height: number, scale_factor: number): void;
    block_state(): BlockState | undefined;
    can(message: Message): boolean;
    /**
     * The `id` of every local changeset (its first op's `actor:clock`), read straight
     * from the graph — `O(#changesets)`. Callers that only need the id set must use
     * this instead of `missing_changesets_tolerant(&[])` + `split_changesets`, which
     * walk, clone, and re-encode the entire history on every push cycle.
     */
    changeset_ids(): string[];
    character_counts(): CharacterCounts;
    copy_selection(): ClipboardPayload | undefined;
    current_heads(): Uint8Array;
    cursor(): CursorMetrics | undefined;
    cursor_hit_test(page: number, x: number, y: number): boolean;
    detach_surface(page: number): void;
    enqueue(message: Message): void;
    export_page_vector(page: number, scale_factor: number): Uint8Array;
    external_elements(): ExternalElement[];
    find_matches(query: string, options?: SearchOptions | null): Selection[];
    freeze_selection(selection: Selection): StableSelection | undefined;
    ime(before_limit: number, after_limit: number): Ime;
    insert_template_fragment(changesets: Uint8Array): void;
    inspect_state(options?: InspectStateOptions | null): string;
    inspect_state_as_macro(): string;
    interactive_hit_test(page: number, x: number, y: number): InteractiveHit | undefined;
    invalidate_surface(page: number): void;
    last_history_tag(): HistoryTag | undefined;
    link_hit_test(page: number, x: number, y: number): LinkRect | undefined;
    link_rects(): LinkRect[];
    local_changesets_since(remote_heads_payload: Uint8Array): Uint8Array;
    materialize_at(heads: Uint8Array): PlainDoc;
    missing_changesets_tolerant(remote_heads_payload: Uint8Array): Uint8Array;
    modifier_span_selection(pos: Position, modifier_type: ModifierType): Selection | undefined;
    modifier_state(): ModifierState | undefined;
    /**
     * Fixed per-page backing sizes for the incremental renderer. Surfaces are
     * allocated at this size (>= any content height) so content-height changes
     * never resize (and clear) the canvas. See `View::page_backing_sizes`.
     */
    page_backing_sizes(): Size[];
    page_external_elements(page: number): ExternalElement[];
    page_link_rects(page: number): LinkRect[];
    page_sizes(): Size[];
    page_table_overlays(page: number): TableOverlay[];
    partition_remote_changesets(payload: Uint8Array): PartitionedChangesets;
    placeholder(): PlaceholderMetrics | undefined;
    pointer_style(page: number, x: number, y: number, read_only: boolean): PointerStyle;
    prose_text(): string;
    prose_to_selection(start: number, end: number): Selection | undefined;
    receive_remote_changeset(payload: Uint8Array): void;
    /**
     * Returns whether a new frame was presented. `false` means no frame will arrive
     * from this call — the page's pixels already match the current state — so hosts
     * that wait for a present (the mobile settle handshake) must treat the page as
     * settled instead of waiting.
     */
    render_surface(page: number): boolean;
    resize_surface(page: number, width: number, height: number, scale_factor: number): void;
    root_attrs(): PlainRootNode;
    root_modifiers(): Modifier[];
    selection(): Selection | undefined;
    selection_endpoints(): SelectionEndpoints | undefined;
    selection_hit_test(page: number, x: number, y: number): boolean;
    set_doc(plain: PlainDoc): void;
    split_changesets(payload: Uint8Array): ChangesetEntry[];
    table_overlays(): TableOverlay[];
    tick(): EditorEvent[];
    tracked_ranges(group?: string | null): TrackedRange[];
    tracked_ranges_at(page: number, x: number, y: number, group?: string | null): TrackedRangeHit[];
    tracked_ranges_containing_position(position: Position, group?: string | null): TrackedRangeEndpoints[];
}

declare class EditorHost {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    add_font_base(family: string, weight: number, data: Uint8Array): void;
    add_font_chunk(family: string, weight: number, chunk_id: number, data: Uint8Array): void;
    static create(icu_data: Uint8Array): EditorHost;
    create_editor_from_doc(doc: PlainDoc, viewport: Viewport): Editor;
    create_editor_from_graph(changesets: Uint8Array, viewport: Viewport): Editor;
    create_editor_from_graph_with_pending(server: Uint8Array, pending_encoded: Uint8Array, viewport: Viewport): Editor;
    extract_text_from_graph(changesets: Uint8Array): string;
    graph_heads(changesets: Uint8Array): Uint8Array;
    root_attrs_from_graph(changesets: Uint8Array): PlainRootNode;
    root_modifiers_from_graph(changesets: Uint8Array): Modifier[];
    set_auto_surround_enabled(enabled: boolean): void;
    set_fonts(families: FontFamily[]): void;
    set_gl_canary(callback: Function): void;
    set_text_replacement_rules(rules: RawTextReplacementRule[]): void;
    set_theme_variant(variant: ThemeVariant): boolean;
}

export type { Editor, EditorHost };

export function createInstance(wasmModule: WebAssembly.Module): Promise<{
    Editor: typeof Editor;
    EditorHost: typeof EditorHost;
}>;
