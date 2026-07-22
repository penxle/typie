use crate::NodeType;

use super::SchemaError;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentExpr {
    Empty,
    /// Accepts any sequence of children unconditionally (never leaf, never
    /// invalid) — used for a node whose true content model is unknowable
    /// (the `Unknown` placeholder), so its children are never validated,
    /// repaired, or dropped.
    Any,
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
            Self::Empty | Self::Any => {}
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

    /// Returns the deterministic additions needed to make `supplied` valid.
    /// Existing nodes are never removed, reordered, or replaced.
    pub fn completion_insertions(&self, supplied: &[NodeType]) -> Option<Vec<(usize, NodeType)>> {
        if self.matches_sequence(supplied) {
            return Some(vec![]);
        }

        let insertions = self.compute_insertions(supplied);
        let mut completed = supplied.to_vec();
        for &(index, node_type) in &insertions {
            if index > completed.len() {
                return None;
            }
            completed.insert(index, node_type);
        }

        self.matches_sequence(&completed).then_some(insertions)
    }

    fn compute_insertions(&self, supplied: &[NodeType]) -> Vec<(usize, NodeType)> {
        match self {
            Self::Empty | Self::Any | Self::ZeroOrMore(_) | Self::Optional(_) => vec![],
            Self::Single(node_type) => supplied
                .is_empty()
                .then_some((0, *node_type))
                .into_iter()
                .collect(),
            Self::OneOrMore(inner) => supplied
                .is_empty()
                .then(|| (0, Self::first_type(inner)))
                .into_iter()
                .collect(),
            Self::Choice(choices) => supplied
                .is_empty()
                .then(|| (0, Self::first_type(&choices[0])))
                .into_iter()
                .collect(),
            Self::Seq(exprs) => Self::compute_seq_insertions(exprs, supplied),
        }
    }

    fn compute_seq_insertions(
        exprs: &[ContentExpr],
        supplied: &[NodeType],
    ) -> Vec<(usize, NodeType)> {
        let mut insertions = Vec::new();
        let mut supplied_index = 0;

        for expr in exprs {
            match expr {
                Self::Single(node_type) => {
                    if supplied.get(supplied_index) == Some(node_type) {
                        supplied_index += 1;
                    } else {
                        insertions.push((supplied_index + insertions.len(), *node_type));
                    }
                }
                Self::ZeroOrMore(inner) | Self::OneOrMore(inner) => {
                    let mut consumed = 0;
                    while supplied
                        .get(supplied_index)
                        .is_some_and(|node_type| inner.matches(*node_type))
                    {
                        supplied_index += 1;
                        consumed += 1;
                    }

                    if matches!(expr, Self::OneOrMore(_)) && consumed == 0 {
                        insertions
                            .push((supplied_index + insertions.len(), Self::first_type(inner)));
                    }
                }
                Self::Optional(inner)
                    if supplied
                        .get(supplied_index)
                        .is_some_and(|node_type| inner.matches(*node_type)) =>
                {
                    supplied_index += 1;
                }
                Self::Optional(_) => {}
                _ => {}
            }
        }

        insertions
    }

    fn first_type(expr: &ContentExpr) -> NodeType {
        match expr {
            Self::Single(node_type) => *node_type,
            Self::Choice(choices) => Self::first_type(&choices[0]),
            Self::OneOrMore(inner) | Self::ZeroOrMore(inner) | Self::Optional(inner) => {
                Self::first_type(inner)
            }
            Self::Seq(exprs) => Self::first_type(&exprs[0]),
            Self::Empty | Self::Any => unreachable!("Empty/Any content has no type"),
        }
    }

    /// Whether `node_type` may appear freely (any count, any position relative
    /// to peers in the same repeatable group) — i.e. it is inside a `ZeroOrMore`
    /// or `OneOrMore`. Types only reachable through `Single`/`Optional`/fixed
    /// `Seq` slots (e.g. a trailing `PageBreak?`) are position-constrained and
    /// return false, so a splice involving them must not skip normalization.
    pub fn is_repeatable(&self, node_type: NodeType) -> bool {
        match self {
            Self::Empty | Self::Any | Self::Single(_) => false,
            Self::Optional(inner) => inner.is_repeatable(node_type),
            Self::ZeroOrMore(inner) | Self::OneOrMore(inner) => {
                inner.matches(node_type) || inner.is_repeatable(node_type)
            }
            Self::Choice(exprs) | Self::Seq(exprs) => {
                exprs.iter().any(|e| e.is_repeatable(node_type))
            }
        }
    }

    pub fn matches(&self, node_type: NodeType) -> bool {
        match self {
            Self::Empty => false,
            Self::Any => true,
            Self::Single(t) => *t == node_type,
            Self::Choice(choices) => choices.iter().any(|choice| choice.matches(node_type)),
            Self::ZeroOrMore(expr) => expr.matches(node_type),
            Self::OneOrMore(expr) => expr.matches(node_type),
            Self::Optional(expr) => expr.matches(node_type),
            Self::Seq(exprs) => exprs.iter().any(|expr| expr.matches(node_type)),
        }
    }

    /// Whether the model can ever absorb a child beyond a sequence it already
    /// matches — false for fixed-arity models (every slot single and required,
    /// e.g. Fold's `FoldTitle, FoldContent`), where any extra child is a
    /// permanent misfit the projection will never render.
    pub fn admits_additional_child(&self) -> bool {
        match self {
            Self::Empty | Self::Single(_) => false,
            Self::Any | Self::ZeroOrMore(_) | Self::OneOrMore(_) | Self::Optional(_) => true,
            Self::Choice(choices) => choices.iter().any(Self::admits_additional_child),
            Self::Seq(exprs) => exprs.iter().any(Self::admits_additional_child),
        }
    }

    pub fn min_required(&self) -> usize {
        match self {
            Self::Empty | Self::Any => 0,
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
            Self::Any => Ok(()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_additional_child_is_false_only_for_fixed_arity_models() {
        assert!(!NodeType::Fold.spec().content.admits_additional_child());
        assert!(NodeType::Root.spec().content.admits_additional_child());
        assert!(
            NodeType::BulletList
                .spec()
                .content
                .admits_additional_child()
        );
        assert!(NodeType::ListItem.spec().content.admits_additional_child());
        assert!(NodeType::TableRow.spec().content.admits_additional_child());
        assert!(NodeType::Table.spec().content.admits_additional_child());
    }

    #[test]
    fn completion_insertions_returns_empty_for_valid_sequence() {
        let content = &NodeType::Fold.spec().content;

        assert_eq!(
            content.completion_insertions(&[NodeType::FoldTitle, NodeType::FoldContent]),
            Some(vec![])
        );
    }

    #[test]
    fn completion_insertions_adds_required_children_without_reordering_supplied_nodes() {
        let fold = &NodeType::Fold.spec().content;
        let list_item = &NodeType::ListItem.spec().content;

        assert_eq!(
            fold.completion_insertions(&[NodeType::FoldTitle]),
            Some(vec![(1, NodeType::FoldContent)])
        );
        assert_eq!(
            list_item.completion_insertions(&[NodeType::BulletList]),
            Some(vec![(0, NodeType::Paragraph)])
        );
    }

    #[test]
    fn completion_insertions_rejects_sequences_that_require_removal_or_reordering() {
        let paragraph = &NodeType::Paragraph.spec().content;
        let fold = &NodeType::Fold.spec().content;

        assert_eq!(
            paragraph.completion_insertions(&[NodeType::PageBreak, NodeType::Text]),
            None
        );
        assert_eq!(
            fold.completion_insertions(&[NodeType::FoldContent, NodeType::FoldTitle]),
            None
        );
    }
}
