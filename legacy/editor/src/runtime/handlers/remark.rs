use crate::model::*;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_add_remark(
        &mut self,
        node_id: String,
        user_id: String,
        text: String,
        created_at: i64,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };
        let remark = Remark {
            id: NodeId::new(),
            user_id,
            text,
            created_at,
        };
        self.transact(move |tr| {
            tr.add_remark(node_id, &remark)?;
            Ok(true)
        })
    }

    pub fn handle_update_remark(
        &mut self,
        node_id: String,
        remark_id: String,
        text: String,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };
        let Some(remark_id) = NodeId::from_string(&remark_id) else {
            return vec![];
        };
        self.transact(move |tr| {
            tr.update_remark(node_id, remark_id, &text)?;
            Ok(true)
        })
    }

    pub fn handle_remove_remark(&mut self, node_id: String, remark_id: String) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };
        let Some(remark_id) = NodeId::from_string(&remark_id) else {
            return vec![];
        };
        self.transact(move |tr| {
            tr.remove_remark(node_id, remark_id)?;
            Ok(true)
        })
    }
}
