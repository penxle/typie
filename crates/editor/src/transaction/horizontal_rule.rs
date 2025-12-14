use crate::model::{HorizontalRuleNode, HorizontalRuleVariant, Node};
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn insert_horizontal_rule(&mut self, variant: HorizontalRuleVariant) -> Result<bool> {
        let node = Node::HorizontalRule(HorizontalRuleNode { variant });
        self.insert_node(node)
    }
}
