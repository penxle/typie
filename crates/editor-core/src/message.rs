pub use editor_common::{Axis, Direction, Movement};
use editor_macros::ffi;
use editor_model::{Fragment, Modifier, ModifierType, Node, NodeId};
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
    Toggle { modifier_type: ModifierType },
    Set { modifier: Modifier },
    ClearAll,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionOp {
    All,
    Set { selection: Selection },
    SetFlat { start: usize, end: usize },
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
    },
    SetColumnWidths {
        widths: Vec<f32>,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeOp {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: Node },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipboardOp {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompositionOp {
    Update {
        text: String,
        replace_length: Option<usize>,
    },
    SetRegion {
        start: usize,
        end: usize,
    },
    Commit {
        text: String,
    },
    CommitAsIs,
    Cancel,
    Flat {
        ops: Vec<FlatImeOp>,
    },
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
    MoveCursor { delta: i32 },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Key { event: KeyEvent },
    Pointer { event: PointerEvent },
    Insertion { op: InsertionOp },
    Deletion { op: DeletionOp },
    Selection { op: SelectionOp },
    Modifier { op: ModifierOp },
    Node { op: NodeOp },
    Clipboard { op: ClipboardOp },
    Composition { op: CompositionOp },
    Navigation { op: NavigationOp },
    History { op: HistoryOp },
    System { event: SystemEvent },
}
