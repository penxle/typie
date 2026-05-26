pub use editor_common::{Axis, Direction, Movement};
use editor_macros::ffi;
use editor_model::{Fragment, Modifier, ModifierType, NodeId, PlainNode, TableBorderStyle};
use editor_state::Selection;
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PointerEvent {
    Down {
        page: usize,
        x: f32,
        y: f32,
        count: u32,
        #[serde(default)]
        modifiers: InputModifiers,
    },
    Move {
        page: usize,
        x: f32,
        y: f32,
    },
    Up,
    Cancel,
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
pub enum SelectionOp {
    Set {
        selection: Selection,
    },
    Unset,
    SetFlat {
        start: usize,
        end: usize,
    },
    ExtendTo {
        anchor_page: usize,
        anchor_x: f32,
        anchor_y: f32,
        head_page: usize,
        head_x: f32,
        head_y: f32,
        initial_selection: Option<Selection>,
    },
    Expand {
        unit: SelectionExpansionUnit,
    },
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
pub enum ViewOp {
    ToggleFold { id: NodeId },
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Key {
        event: KeyEvent,
    },
    Pointer {
        event: PointerEvent,
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
    Navigation {
        op: NavigationOp,
    },
    History {
        op: HistoryOp,
    },
    System {
        event: SystemEvent,
    },

    #[ffi(skip)]
    Remote {
        changeset: editor_crdt::Changeset<editor_model::DocOp>,
    },
}
