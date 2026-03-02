use crate::model::{HorizontalRuleNode, HorizontalRuleVariant, Node};
use crate::state::selection_helpers::selected_single_block_id;
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn insert_horizontal_rule(&mut self, variant: HorizontalRuleVariant) -> Result<bool> {
        let node = Node::HorizontalRule(HorizontalRuleNode { variant });
        self.insert_node(node)
    }

    pub fn set_horizontal_rule(&mut self, variant: HorizontalRuleVariant) -> Result<bool> {
        if self.update_selected_horizontal_rule_variant(variant)? {
            return Ok(true);
        }

        self.insert_horizontal_rule(variant)
    }

    fn update_selected_horizontal_rule_variant(
        &mut self,
        variant: HorizontalRuleVariant,
    ) -> Result<bool> {
        let selection = self.selection().clone();
        let Some(node_id) = selected_single_block_id(self.doc(), &selection) else {
            return Ok(false);
        };
        let Some(selected_node) = self.node(node_id) else {
            return Ok(false);
        };

        let current_variant = match selected_node.node() {
            Some(Node::HorizontalRule(node)) => node.variant,
            _ => return Ok(false),
        };

        if current_variant == variant {
            return Ok(true);
        }

        let Some(node_ref) = self.node_mut(node_id) else {
            return Ok(false);
        };

        node_ref.as_mut().update(|node| {
            if let Node::HorizontalRule(horizontal_rule) = node {
                horizontal_rule.variant = variant;
            }
        })?;
        self.mark_attr_mutation(node_id);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{HorizontalRuleVariant, NodeId};
    use crate::types::Affinity;

    #[test]
    fn insert_horizontal_rule_inserts_even_when_horizontal_rule_selected() {
        let mut hr = id!();

        let initial = state! {
            doc {
                paragraph { text { "a" } }
                @hr horizontal_rule {}
                paragraph { text { "b" } }
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_horizontal_rule(HorizontalRuleVariant::Diamond)
            .unwrap());

        let expected = state! {
            doc {
                paragraph { text { "a" } }
                @hr horizontal_rule {}
                horizontal_rule(variant: HorizontalRuleVariant::Diamond,) {}
                paragraph { text { "b" } }
            }
            selection { (NodeId::ROOT, 2, Affinity::Downstream) -> (NodeId::ROOT, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_horizontal_rule_updates_selected_horizontal_rule_variant() {
        let mut hr = id!();

        let initial = state! {
            doc {
                paragraph { text { "a" } }
                @hr horizontal_rule {}
                paragraph { text { "b" } }
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        let actual = transact!(initial, |tr| tr
            .set_horizontal_rule(HorizontalRuleVariant::Diamond)
            .unwrap());

        let expected = state! {
            doc {
                paragraph { text { "a" } }
                @hr horizontal_rule(variant: HorizontalRuleVariant::Diamond,) {}
                paragraph { text { "b" } }
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_horizontal_rule_inserts_when_not_selected() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .set_horizontal_rule(HorizontalRuleVariant::Diamond)
            .unwrap());

        let expected = state! {
            doc {
                horizontal_rule(variant: HorizontalRuleVariant::Diamond,) {}
                paragraph { text { "hello" } }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }
}
