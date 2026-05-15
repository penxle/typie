use super::Schema;
use super::content::ContentExpr;
use super::context::ContextExpr;

#[derive(Debug, Clone)]
pub struct NodeSpec {
    pub content: ContentExpr,
    pub context: ContextExpr,
    pub inline: bool,
    pub selectable: bool,
    pub isolating: bool,
    /// When true, the node is a structural part of its parent and cannot be deleted alone; only its content can be cleared.
    pub structural: bool,
    pub external: bool,
    pub monolithic: bool,
}

impl NodeSpec {
    pub fn is_textblock(&self) -> bool {
        let allowed = self.content.allowed_types();
        if allowed.is_empty() {
            return false;
        }
        allowed.iter().all(|t| Schema::node_spec(*t).inline)
    }

    pub fn is_leaf(&self) -> bool {
        self.content.is_leaf()
    }
}

impl Default for NodeSpec {
    fn default() -> Self {
        Self {
            content: ContentExpr::Empty,
            context: ContextExpr::Any,
            inline: false,
            selectable: false,
            isolating: false,
            structural: false,
            external: false,
            monolithic: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModifierSpec {
    pub context: ContextExpr,
    pub expand: Expand,
    pub overlap: bool,
    pub inheritable: bool,
}

impl Default for ModifierSpec {
    fn default() -> Self {
        Self {
            context: ContextExpr::Any,
            expand: Expand::After,
            overlap: false,
            inheritable: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expand {
    Before,
    After,
    Both,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeType;

    #[test]
    fn is_leaf_classifies_nodes() {
        assert!(Schema::node_spec(NodeType::Text).is_leaf());
        assert!(Schema::node_spec(NodeType::HardBreak).is_leaf());
        assert!(Schema::node_spec(NodeType::PageBreak).is_leaf());
        assert!(Schema::node_spec(NodeType::Image).is_leaf());
        assert!(Schema::node_spec(NodeType::HorizontalRule).is_leaf());
        assert!(!Schema::node_spec(NodeType::Root).is_leaf());
        assert!(!Schema::node_spec(NodeType::Paragraph).is_leaf());
        assert!(!Schema::node_spec(NodeType::Blockquote).is_leaf());
        assert!(!Schema::node_spec(NodeType::Table).is_leaf());
    }

    #[test]
    fn monolithic_flag_per_node_type() {
        for ty in [
            NodeType::Blockquote,
            NodeType::Callout,
            NodeType::Fold,
            NodeType::Table,
            NodeType::HorizontalRule,
        ] {
            assert!(
                Schema::node_spec(ty).monolithic,
                "{ty:?} must be monolithic"
            );
        }
        for ty in [
            NodeType::Root,
            NodeType::Paragraph,
            NodeType::ListItem,
            NodeType::FoldTitle,
            NodeType::FoldContent,
            NodeType::TableRow,
            NodeType::TableCell,
            NodeType::BulletList,
            NodeType::OrderedList,
            NodeType::Text,
        ] {
            assert!(
                !Schema::node_spec(ty).monolithic,
                "{ty:?} must not be monolithic"
            );
        }
    }
}
