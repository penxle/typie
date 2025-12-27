use crate::layout::interactive::InteractionKind;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_interaction(&mut self, kind: InteractionKind) -> Vec<Effect> {
        match kind {
            InteractionKind::Toggle { node_id } => self.toggle_view_state(node_id),
            InteractionKind::CycleCalloutVariant { node_id } => self.cycle_callout_variant(node_id),
        }
    }
}
