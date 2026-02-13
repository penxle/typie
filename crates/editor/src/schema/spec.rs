use crate::model::{AnnotationType, NodeType, StyleType};
use crate::schema::Schema;
use crate::schema::content::ContentExpr;

#[derive(Debug, Clone)]
pub struct NodeSpec {
    pub content: ContentExpr,
    pub styles: Option<&'static [StyleType]>,
    pub annotations: Option<&'static [AnnotationType]>,
    pub inline: bool,
    pub selectable: bool,
    pub isolating: bool,
    pub structural: bool, // 부모의 구조적 일부인 노드. true면 부모 없이 단독 삭제 불가, 내용만 삭제됨.
    pub external: bool,
    pub grandparent_must_be: Option<NodeType>,
}

impl NodeSpec {
    pub fn is_textblock(&self, schema: &Schema) -> bool {
        let allowed = self.content.allowed_types();
        if allowed.is_empty() {
            return false;
        }
        allowed.iter().all(|t| schema.node_spec(*t).inline)
    }

    pub fn is_structural_root(&self, schema: &Schema) -> bool {
        if !self.isolating {
            return false;
        }
        if self.structural {
            return false;
        }
        let allowed = self.content.allowed_types();
        if allowed.is_empty() {
            return false;
        }
        allowed.iter().any(|t| schema.node_spec(*t).structural)
    }
}

impl Default for NodeSpec {
    fn default() -> Self {
        Self {
            content: ContentExpr::Empty,
            styles: None,
            annotations: None,
            inline: false,
            selectable: false,
            isolating: false,
            structural: false,
            external: false,
            grandparent_must_be: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyleSpec {
    #[allow(dead_code)]
    pub expand: Expand,
}

impl Default for StyleSpec {
    fn default() -> Self {
        Self {
            expand: Expand::After,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotationSpec {
    pub overlap: bool,
}

impl Default for AnnotationSpec {
    fn default() -> Self {
        Self { overlap: false }
    }
}

#[derive(Debug, Clone)]
pub enum Expand {
    #[allow(dead_code)]
    Before,
    After,
    #[allow(dead_code)]
    Both,
    None,
}
