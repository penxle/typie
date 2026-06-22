use editor_state::StableSelection;
use editor_view::DropTarget;

use crate::message::ExternalDndPayloadKind;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum DndState {
    #[default]
    Idle,
    InternalDnd {
        source: StableSelection,
        drop_target: Option<DropTarget>,
    },
    ExternalDnd {
        payload: ExternalDndPayloadKind,
        drop_target: Option<DropTarget>,
    },
}

impl DndState {
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
