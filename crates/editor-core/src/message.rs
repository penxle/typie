pub use editor_common::{Axis, Direction, Movement};
use editor_macros::ffi;
use editor_model::{Modifier, ModifierType, Node, NodeId, NodeType, TextAlign};
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
pub struct KeyModifiers {
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
    pub modifiers: KeyModifiers,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerButton {
    Primary,
    Auxiliary,
    Secondary,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DragPayload {
    Internal,
    Text(String),
    Html { html: String, text: String },
    Files(Vec<String>),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DragEvent {
    Start {
        x: f32,
        y: f32,
    },
    Over {
        x: f32,
        y: f32,
    },
    Enter,
    Leave,
    End,
    Drop {
        x: f32,
        y: f32,
        payload: DragPayload,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerEvent {
    Down {
        x: f32,
        y: f32,
        count: u32,
        button: PointerButton,
        modifiers: KeyModifiers,
    },
    Move {
        x: f32,
        y: f32,
        buttons: u16,
    },
    Up {
        x: f32,
        y: f32,
        button: PointerButton,
    },
    Drag(DragEvent),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakKind {
    Block,
    Line,
    Page,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertionIntent {
    Text(String),
    Break(BreakKind),
    Block(Node),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionIntent {
    Selection,
    Move(Movement),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormattingIntent {
    ToggleModifier(ModifierType),
    SetModifier(Modifier),
    Clear,
    SetTextAlign(TextAlign),
    SetLineHeight(u32),
    ToggleWrap(NodeType),
    Indent,
    Outdent,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionIntent {
    All,
    Set(Selection),
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum NodeIntent {
    Delete { id: NodeId },
    SetAttrs { id: NodeId, attrs: Node },
    ToggleFold { id: NodeId },
    Table { id: NodeId, op: TableOp },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardIntent {
    Paste { html: Option<String>, text: String },
    Cut,
    Copy,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionIntent {
    Update {
        text: String,
        replace_length: Option<usize>,
    },
    End,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationIntent {
    Move { movement: Movement, extend: bool },
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryIntent {
    Undo,
    Redo,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub struct FontMapping {
    pub family: String,
    pub weight: u16,
    pub codepoints: Vec<u32>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemEvent {
    Initialize,
    Resize {
        width: f32,
        height: f32,
        scale_factor: f64,
    },
    SetFocused(bool),
    FontsLoaded {
        family: String,
        weight: u16,
        mappings: Vec<FontMapping>,
    },
    SetExternalHeight {
        node_id: NodeId,
        height: f32,
    },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Key(KeyEvent),
    Pointer(PointerEvent),
    Intent(Intent),
    System(SystemEvent),
}
