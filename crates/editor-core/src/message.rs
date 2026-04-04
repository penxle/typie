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
pub enum InsertionIntent {
    Text { text: String },
    Break { kind: Break },
    Node { node: Node },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeletionIntent {
    Selection,
    Move { movement: Movement },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FormattingIntent {
    ToggleModifier { modifier_type: ModifierType },
    SetModifier { modifier: Modifier },
    ClearModifiers,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionIntent {
    All,
    Set { selection: Selection },
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
pub enum NodeIntent {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: Node },
    ToggleFold { id: NodeId },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipboardIntent {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompositionIntent {
    Update {
        text: String,
        replace_length: Option<usize>,
    },
    End,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Intent {
    Insertion { intent: InsertionIntent },
    Deletion { intent: DeletionIntent },
    Formatting { intent: FormattingIntent },
    Selection { intent: SelectionIntent },
    Node { intent: NodeIntent },
    Clipboard { intent: ClipboardIntent },
    Composition { intent: CompositionIntent },
    Navigation { intent: NavigationIntent },
    History { intent: HistoryIntent },
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Key { event: KeyEvent },
    Pointer { event: PointerEvent },
    Intent { intent: Intent },
    System { event: SystemEvent },
}
