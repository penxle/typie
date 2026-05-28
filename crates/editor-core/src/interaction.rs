use editor_state::{Position, Selection, StableSelection};
use editor_view::DropTarget;

use crate::message::ExternalDndPayloadKind;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PressContext {
    Empty,
    InSelection,
    OnSelectable(Selection),
}

impl PressContext {
    pub(crate) fn can_drag_content(&self) -> bool {
        matches!(self, Self::InSelection | Self::OnSelectable(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InteractionState {
    Idle,
    Pressed {
        page: usize,
        start_x: f32,
        start_y: f32,
        position: Position,
        selection_anchor: Option<Selection>,
        context: PressContext,
    },
    DraggingSelection {
        anchor: Selection,
    },
    InternalDnd {
        source: StableSelection,
        drop_target: Option<DropTarget>,
    },
    ExternalDnd {
        payload: ExternalDndPayloadKind,
        drop_target: Option<DropTarget>,
    },
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::Idle
    }
}

impl InteractionState {
    pub(crate) fn drop_target(&self) -> Option<DropTarget> {
        match self {
            Self::InternalDnd { drop_target, .. } | Self::ExternalDnd { drop_target, .. } => {
                *drop_target
            }
            _ => None,
        }
    }

    pub(crate) fn set_drop_target(&mut self, target: Option<DropTarget>) {
        match self {
            Self::InternalDnd { drop_target, .. } | Self::ExternalDnd { drop_target, .. } => {
                *drop_target = target;
            }
            _ => {}
        }
    }
}
