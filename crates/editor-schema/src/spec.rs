use editor_model::NodeType;

use crate::Schema;
use crate::content::ContentExpr;
use crate::context::ContextExpr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockSelectionBoundaryMode {
    FrontOnly,
    FrontOrBack,
    Both,
}

#[derive(Debug, Clone)]
pub struct NodeSpec {
    pub content: ContentExpr,
    pub context: ContextExpr,
    pub inline: bool,
    pub selectable: bool,
    pub isolating: bool,
    pub structural: bool, // 부모의 구조적 일부인 노드. true면 부모 없이 단독 삭제 불가, 내용만 삭제됨.
    pub external: bool,
    pub promote_item_type_on_delete: Option<NodeType>,
    pub block_selection_boundary_mode: Option<BlockSelectionBoundaryMode>,
}

impl NodeSpec {
    pub fn is_textblock(&self) -> bool {
        let allowed = self.content.allowed_types();
        if allowed.is_empty() {
            return false;
        }
        allowed.iter().all(|t| Schema::node_spec(*t).inline)
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
            promote_item_type_on_delete: None,
            block_selection_boundary_mode: None,
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
