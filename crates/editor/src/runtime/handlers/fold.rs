use crate::model::NodeId;
use crate::runtime::{Effect, NodeViewState, Runtime};

impl Runtime {
    pub(crate) fn toggle_view_state(&mut self, node_id: NodeId) -> Vec<Effect> {
        let current_expanded = self
            .view_states
            .get(&node_id)
            .map(|s| s.fold_expanded())
            .unwrap_or(false);

        self.view_states.insert(
            node_id,
            NodeViewState::Fold {
                expanded: !current_expanded,
            },
        );

        self.layout_cache.borrow_mut().invalidate_all();

        vec![Effect::LayoutChanged]
    }

    pub(crate) fn handle_toggle_fold_expansion(&mut self, node_id: String) -> Vec<Effect> {
        let Some(id) = NodeId::from_string(&node_id) else {
            return vec![];
        };
        self.toggle_view_state(id)
    }

    pub(crate) fn handle_insert_fold(&mut self) -> Vec<Effect> {
        let mut created_fold_id = None;
        let mut effects = self.transact(|tr| {
            let fold_id = tr.insert_fold()?;
            created_fold_id = fold_id;
            Ok(fold_id.is_some())
        });

        if let Some(fold_id) = created_fold_id {
            effects.extend(self.toggle_view_state(fold_id));
        }

        effects
    }

    pub(crate) fn handle_unwrap_fold(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.unwrap_fold())
    }
}
