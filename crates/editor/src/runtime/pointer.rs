use crate::layout::interactive::InteractionKind;
use crate::state::{Position, Selection};

#[derive(Debug, Clone, PartialEq)]
pub enum PressContext {
    Empty,
    InSelection,
    OnSelectable(Selection),
    Interactive(InteractionKind),
}

impl PressContext {
    pub fn can_drag_content(&self) -> bool {
        matches!(self, Self::InSelection | Self::OnSelectable(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PointerMode {
    Idle,
    Pressed {
        page_idx: usize,
        start_x: f32,
        start_y: f32,
        document_position: Position,
        context: PressContext,
    },
    DraggingContent,
    DraggingExternal,
    DraggingSelection,
}

impl PointerMode {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

impl Default for PointerMode {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct PointerState {
    pub(crate) mode: PointerMode,
    pub(crate) drop_target: Option<Position>,
}

impl PointerState {
    pub fn reset(&mut self) {
        self.mode = PointerMode::Idle;
        self.drop_target = None;
    }

    pub fn is_dragging_content(&self) -> bool {
        matches!(self.mode, PointerMode::DraggingContent)
    }

    pub fn is_dragging_external(&self) -> bool {
        matches!(self.mode, PointerMode::DraggingExternal)
    }
}
