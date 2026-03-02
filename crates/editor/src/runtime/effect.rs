use crate::model::{ParagraphNode, Style};
use crate::runtime::text_replacement::ReplacementUndoState;
use crate::state::Selection;
use crate::{model::NodeId, state::Position, types::PointerStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MutationKind {
    ViewState,
    Attr,
    Text,
    Structure,
    UnknownRemote,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    DocChanged,
    NodeMutated {
        node_id: NodeId,
        kind: MutationKind,
    },
    SelectionChanged,
    PendingStylesChanged,
    SettingsChanged,
    FullLayoutInvalidation,
    LayoutChanged,
    PreeditChanged {
        node_id: Option<NodeId>,
    },
    FontDetected {
        family: String,
        weight: u16,
        codepoints: Vec<u32>,
    },
    ExternalElementChanged,
    PointerStyleChanged {
        style: PointerStyle,
    },
    DropTargetChanged {
        target: Option<Position>,
    },
    ExitedDocumentStart,
    TextReplacementApplied {
        undo_state: ReplacementUndoState,
    },
    HtmlPasted {
        selection: Selection,
        text: String,
        styles: Vec<Style>,
        paragraph_attrs: Option<ParagraphNode>,
    },
}

impl Effect {
    pub fn priority(&self) -> u8 {
        match self {
            Effect::DocChanged => 0,
            Effect::NodeMutated { .. } => 1,
            Effect::SelectionChanged => 3,
            Effect::PendingStylesChanged => 4,
            Effect::SettingsChanged => 5,
            Effect::FullLayoutInvalidation => 6,
            Effect::LayoutChanged => 7,
            Effect::PreeditChanged { .. } => 8,
            Effect::HtmlPasted { .. } => 9,
            Effect::FontDetected { .. } => 10,
            Effect::ExternalElementChanged => 11,
            Effect::PointerStyleChanged { .. } => 12,
            Effect::DropTargetChanged { .. } => 13,
            Effect::ExitedDocumentStart => 14,
            Effect::TextReplacementApplied { .. } => 15,
        }
    }
}
