use editor_crdt::Dot;
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
        reuse_node_id: Option<Dot>,
    },
}

impl DndState {
    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        match self {
            Self::InternalDnd { drop_target, .. } | Self::ExternalDnd { drop_target, .. } => {
                drop_target.as_ref()
            }
            _ => None,
        }
    }

    pub(crate) fn set_over_target(
        &mut self,
        target: Option<DropTarget>,
        reuse_node_id: Option<Dot>,
    ) {
        match self {
            Self::InternalDnd { drop_target, .. } => {
                *drop_target = target;
            }
            Self::ExternalDnd {
                drop_target,
                reuse_node_id: active_reuse_node_id,
                ..
            } => {
                *drop_target = target;
                *active_reuse_node_id = reuse_node_id;
            }
            _ => {}
        }
    }

    pub(crate) fn reuse_node_id(&self) -> Option<Dot> {
        match self {
            Self::ExternalDnd { reuse_node_id, .. } => *reuse_node_id,
            _ => None,
        }
    }
}
