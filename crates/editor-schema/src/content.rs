use editor_model::NodeType;

use crate::SchemaError;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentExpr {
    Empty,
    Single(NodeType),
    Seq(Vec<ContentExpr>),
    Choice(Vec<ContentExpr>),
    ZeroOrMore(Box<ContentExpr>),
    OneOrMore(Box<ContentExpr>),
    Optional(Box<ContentExpr>),
}

impl ContentExpr {
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn allowed_types(&self) -> Vec<NodeType> {
        let mut types = Vec::new();
        self.collect_types(&mut types);
        types
    }

    fn collect_types(&self, types: &mut Vec<NodeType>) {
        match self {
            Self::Empty => {}
            Self::Single(t) => types.push(*t),
            Self::Choice(choices) => {
                for choice in choices {
                    choice.collect_types(types);
                }
            }
            Self::ZeroOrMore(expr) | Self::OneOrMore(expr) | Self::Optional(expr) => {
                expr.collect_types(types);
            }
            Self::Seq(exprs) => {
                for expr in exprs {
                    expr.collect_types(types);
                }
            }
        }
    }

    pub fn matches_sequence(&self, types: &[NodeType]) -> bool {
        self.validate(types).is_ok()
    }

    pub fn matches(&self, node_type: NodeType) -> bool {
        match self {
            Self::Empty => false,
            Self::Single(t) => *t == node_type,
            Self::Choice(choices) => choices.iter().any(|choice| choice.matches(node_type)),
            Self::ZeroOrMore(expr) => expr.matches(node_type),
            Self::OneOrMore(expr) => expr.matches(node_type),
            Self::Optional(expr) => expr.matches(node_type),
            Self::Seq(exprs) => exprs.iter().any(|expr| expr.matches(node_type)),
        }
    }

    pub fn min_required(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::ZeroOrMore(_) => 0,
            Self::Optional(_) => 0,
            Self::Single(_) => 1,
            Self::OneOrMore(_) => 1,
            Self::Choice(_) => 1,
            Self::Seq(exprs) => exprs.iter().map(|e| e.min_required()).sum(),
        }
    }

    pub fn validate(&self, nodes: &[NodeType]) -> Result<(), SchemaError> {
        match self {
            Self::Empty => {
                if !nodes.is_empty() {
                    return Err(SchemaError::InvalidContent(format!(
                        "Expected no children, got {}",
                        nodes.len()
                    )));
                }

                Ok(())
            }
            Self::Single(expected) => {
                if nodes.len() != 1 {
                    return Err(SchemaError::InvalidContent(format!(
                        "Expected 1 child, got {}",
                        nodes.len()
                    )));
                }

                if nodes[0] != *expected {
                    return Err(SchemaError::InvalidContent(format!(
                        "Expected {:?}, got {:?}",
                        expected, nodes[0]
                    )));
                }

                Ok(())
            }
            Self::Choice(choices) => {
                if nodes.len() != 1 {
                    return Err(SchemaError::InvalidContent(format!(
                        "Choice requires exactly 1 child, got {}",
                        nodes.len()
                    )));
                }

                let node = nodes[0];
                for choice in choices {
                    if choice.matches(node) {
                        return Ok(());
                    }
                }

                Err(SchemaError::InvalidContent(format!(
                    "Node {:?} doesn't match any of the allowed types in choice",
                    node
                )))
            }
            Self::ZeroOrMore(expr) => {
                for &node in nodes {
                    if !expr.matches(node) {
                        return Err(SchemaError::InvalidContent(format!(
                            "Node {:?} doesn't match the allowed type in zero-or-more",
                            node
                        )));
                    }
                }

                Ok(())
            }
            Self::OneOrMore(expr) => {
                if nodes.is_empty() {
                    return Err(SchemaError::InvalidContent(
                        "OneOrMore requires at least 1 child".into(),
                    ));
                }

                for &node in nodes {
                    if !expr.matches(node) {
                        return Err(SchemaError::InvalidContent(format!(
                            "Node {:?} doesn't match the allowed type in one-or-more",
                            node
                        )));
                    }
                }

                Ok(())
            }
            Self::Optional(expr) => {
                if nodes.len() > 1 {
                    return Err(SchemaError::InvalidContent(format!(
                        "Optional allows at most 1 child, got {}",
                        nodes.len()
                    )));
                }

                if nodes.len() == 1 && !expr.matches(nodes[0]) {
                    return Err(SchemaError::InvalidContent(format!(
                        "Node {:?} doesn't match the allowed type in optional",
                        nodes[0]
                    )));
                }

                Ok(())
            }
            Self::Seq(exprs) => {
                let mut node_idx = 0;

                for (i, expr) in exprs.iter().enumerate() {
                    match expr {
                        Self::ZeroOrMore(inner) => {
                            let required_after = exprs[i + 1..]
                                .iter()
                                .map(|e| e.min_required())
                                .sum::<usize>();
                            let max_consume = nodes.len().saturating_sub(required_after);

                            while node_idx < max_consume && inner.matches(nodes[node_idx]) {
                                node_idx += 1;
                            }
                        }
                        Self::OneOrMore(inner) => {
                            if node_idx >= nodes.len() || !inner.matches(nodes[node_idx]) {
                                return Err(SchemaError::InvalidContent(
                                    "OneOrMore requires at least one matching node".into(),
                                ));
                            }

                            node_idx += 1;

                            let required_after = exprs[i + 1..]
                                .iter()
                                .map(|e| e.min_required())
                                .sum::<usize>();
                            let max_consume = nodes.len().saturating_sub(required_after);

                            while node_idx < max_consume && inner.matches(nodes[node_idx]) {
                                node_idx += 1;
                            }
                        }
                        Self::Optional(inner) => {
                            if node_idx < nodes.len() && inner.matches(nodes[node_idx]) {
                                node_idx += 1;
                            }
                        }
                        Self::Single(expected) => {
                            if node_idx >= nodes.len() {
                                return Err(SchemaError::InvalidContent(format!(
                                    "Expected {:?}, but no more children",
                                    expected
                                )));
                            }

                            if nodes[node_idx] != *expected {
                                return Err(SchemaError::InvalidContent(format!(
                                    "Expected {:?}, got {:?}",
                                    expected, nodes[node_idx]
                                )));
                            }

                            node_idx += 1;
                        }
                        _ => {
                            unimplemented!()
                        }
                    }
                }

                if node_idx < nodes.len() {
                    return Err(SchemaError::InvalidContent(format!(
                        "Unexpected extra children: {} nodes remaining",
                        nodes.len() - node_idx
                    )));
                }

                Ok(())
            }
        }
    }
}
