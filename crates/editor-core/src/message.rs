pub use editor_common::{Axis, Direction, Movement};
use editor_macros::ffi;
use editor_model::{Modifier, ModifierType, Node, NodeId};
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
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: InputModifiers,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PointerEvent {
    Down {
        x: f32,
        y: f32,
        count: u32,
        modifiers: InputModifiers,
    },
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
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum InsertionIntent {
    Text(String),
    Break(Break),
    Node(Node),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DeletionIntent {
    Selection,
    Move(Movement),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FormattingIntent {
    ToggleModifier(ModifierType),
    SetModifier(Modifier),
    ClearModifiers,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SelectionIntent {
    All,
    Set(Selection),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
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
    SelectAxis(Option<Axis>),
    SetColumnWidths(Vec<f32>),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NodeIntent {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: Node },
    ToggleFold { id: NodeId },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ClipboardIntent {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CompositionIntent {
    Update {
        text: String,
        replace_length: Option<usize>,
    },
    End,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NavigationIntent {
    Move { movement: Movement, extend: bool },
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryIntent {
    Undo,
    Redo,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Intent {
    Insertion(InsertionIntent),
    Deletion(DeletionIntent),
    Formatting(FormattingIntent),
    Selection(SelectionIntent),
    Node(NodeIntent),
    Clipboard(ClipboardIntent),
    Composition(CompositionIntent),
    Navigation(NavigationIntent),
    History(HistoryIntent),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SystemEvent {
    Initialize,
    Resize {
        width: f32,
        height: f32,
        scale_factor: f64,
    },
    SetFocused(bool),
    FontManifestLoaded {
        family: String,
        weight: u16,
    },
    FontBaseLoaded {
        family: String,
        weight: u16,
    },
    FontChunkLoaded {
        family: String,
        weight: u16,
    },
    SetExternalHeight {
        node_id: NodeId,
        height: f32,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Message {
    Key(KeyEvent),
    Pointer(PointerEvent),
    Intent(Intent),
    System(SystemEvent),
}
