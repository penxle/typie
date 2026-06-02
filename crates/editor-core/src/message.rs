pub use editor_common::{Axis, DecorationStyle, Direction, Movement};
use editor_macros::ffi;
use editor_model::{Fragment, Modifier, ModifierType, NodeId, PlainNode, TableBorderStyle};
use editor_state::{Position, Selection, StableSelection};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Enter,
    Backspace,
    Delete,
    Tab,
    Escape,
}

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InputModifiers {
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KeyEvent {
    pub key: Key,
    #[serde(default)]
    pub modifiers: InputModifiers,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalDndPayloadKind {
    Text,
    Html,
    ImageFiles,
    Files,
    MixedFiles,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DndDropPayload {
    InternalSelection,
    Text { text: String, html: Option<String> },
    Files { image_count: u32, file_count: u32 },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DndOp {
    StartInternalSelection,
    EnterExternal {
        payload: ExternalDndPayloadKind,
    },
    Over {
        page: usize,
        x: f32,
        y: f32,
        #[serde(default)]
        modifiers: InputModifiers,
    },
    Leave,
    Drop {
        page: usize,
        x: f32,
        y: f32,
        payload: DndDropPayload,
        #[serde(default)]
        modifiers: InputModifiers,
    },
    End,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Break {
    Line,
    Paragraph,
    Page,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InsertionOp {
    Text { text: String },
    Break { kind: Break },
    Fragment { fragment: Fragment },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeletionOp {
    Selection,
    Move { movement: Movement },
    Surrounding { before: usize, after: usize },
    SurroundingCodePoints { before: usize, after: usize },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModifierOp {
    Toggle {
        modifier_type: ModifierType,
    },
    Set {
        modifier: Modifier,
    },
    SetOnNode {
        id: NodeId,
        modifier: Modifier,
    },
    Edit {
        modifier_type: ModifierType,
        modifier: Option<Modifier>,
    },
    ClearAll,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StyleOp {
    Apply {
        node_id: NodeId,
        style_id: String,
    },
    Unapply {
        node_id: NodeId,
        style_id: String,
    },
    ApplyToSelection {
        style_id: String,
    },
    UnsetInSelection,
    CreateFromSelection {
        style_id: String,
        name: String,
    },
    UpdateFromSelection,
    Define {
        style_id: String,
        name: String,
        modifiers: Vec<Modifier>,
    },
    Delete {
        style_id: String,
    },
    Rename {
        style_id: String,
        name: String,
    },
    SetModifier {
        style_id: String,
        modifier: Modifier,
    },
    UnsetModifier {
        style_id: String,
        modifier_type: ModifierType,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionOp {
    Set {
        selection: Selection,
    },
    SetFrozen {
        selection: StableSelection,
    },
    Unset,
    SetAt {
        page: usize,
        x: f32,
        y: f32,
    },
    SetFlat {
        start: usize,
        end: usize,
    },
    ExtendTo {
        anchor: Position,
        head_page: usize,
        head_x: f32,
        head_y: f32,
        base_selection: Option<Selection>,
    },
    SelectUnitAt {
        page: usize,
        x: f32,
        y: f32,
        unit: SelectionPointUnit,
    },
    Expand {
        unit: SelectionExpansionUnit,
    },
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionPointUnit {
    Word,
    Sentence,
    Paragraph,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionExpansionUnit {
    Word,
    Sentence,
    Paragraph,
    All,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TableOp {
    InsertAxis {
        axis: Axis,
        index: usize,
        before: bool,
    },
    DeleteAxis {
        axis: Axis,
        index: usize,
    },
    MoveAxis {
        axis: Axis,
        from: usize,
        to: usize,
    },
    SelectAxis {
        axis: Option<Axis>,
        index: Option<usize>,
    },
    SetColumnWidths {
        widths: Vec<f32>,
    },
    SetBorderStyle {
        border_style: TableBorderStyle,
    },
    SetProportion {
        proportion: u32,
    },
    SetAxisBackgroundColor {
        axis: Axis,
        index: usize,
        color: Option<String>,
    },
    SetCellSelectionBackgroundColor {
        color: Option<String>,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeOp {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: PlainNode },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScrollTarget {
    TrackedItem { id: String },
    Selection,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewOp {
    ToggleFold { id: NodeId },
    ScrollIntoView { target: ScrollTarget },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipboardOp {
    Paste { html: Option<String>, text: String },
    RepasteAsText,
    Cut,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NavigationOp {
    Move { movement: Movement, extend: bool },
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryOp {
    Undo,
    Redo,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemEvent {
    Initialize,
    Resize {
        width: f32,
        height: f32,
        scale_factor: f64,
    },
    SetFocused {
        focused: bool,
    },
    ThemeVariantChanged,
    FontBaseLoaded {
        family: String,
        weight: u16,
    },
    FontChunkLoaded {
        family: String,
        weight: u16,
        chunk_id: u16,
    },
    SetExternalHeight {
        node_id: NodeId,
        height: f32,
    },
    FontsChanged,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlatImeOp {
    SetSelection { start: usize, end: usize },
    ReplaceSelection { text: String },
    Compose { text: String },
    DeleteSurrounding { before: usize, after: usize },
    DeleteSurroundingUtf16 { before: usize, after: usize },
    SetComposition { start: usize, end: usize },
    ClearComposition,
    CommitAsIs,
    MoveCursor { delta: i32 },
}

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchOptions {
    #[serde(default)]
    pub match_whole_word: bool,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrackedRangeOp {
    Add {
        id: String,
        group: String,
        selection: Selection,
        #[serde(default)]
        metadata: String,
    },
    AddFrozen {
        id: String,
        group: String,
        selection: editor_state::StableSelection,
        #[serde(default)]
        metadata: String,
    },
    Remove {
        id: String,
    },
    ClearGroup {
        group: String,
    },
    Invalidate {
        id: String,
    },
    SetGroupDecoration {
        group: String,
        style: DecorationStyle,
        enabled: bool,
        #[serde(default)]
        z_index: i32,
    },
    RemoveGroupDecoration {
        group: String,
    },
    ReplaceText {
        id: String,
        #[serde(default)]
        expected_text: Option<String>,
        replacement: String,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Key {
        event: KeyEvent,
    },
    Insertion {
        op: InsertionOp,
    },
    Deletion {
        op: DeletionOp,
    },
    Selection {
        op: SelectionOp,
    },
    Modifier {
        op: ModifierOp,
    },
    Style {
        op: StyleOp,
    },
    Node {
        op: NodeOp,
    },
    View {
        op: ViewOp,
    },
    Clipboard {
        op: ClipboardOp,
    },
    TextInput {
        ops: Vec<FlatImeOp>,
    },
    Dnd {
        op: DndOp,
    },
    Navigation {
        op: NavigationOp,
    },
    History {
        op: HistoryOp,
    },
    System {
        event: SystemEvent,
    },
    TrackedRange {
        op: TrackedRangeOp,
    },

    #[ffi(skip)]
    Remote {
        changeset: editor_crdt::Changeset<editor_model::DocOp>,
    },
}
