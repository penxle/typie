use crate::model::NodeType;
use anyhow::Result;

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

    pub fn allows_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::ZeroOrMore(_) => true,
            Self::Optional(_) => true,
            Self::Single(_) => false,
            Self::OneOrMore(_) => false,
            Self::Choice(_) => false,
            Self::Seq(exprs) => exprs.iter().all(|expr| expr.allows_empty()),
        }
    }

    fn min_required(&self) -> usize {
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

    pub fn validate(&self, nodes: &[NodeType]) -> Result<()> {
        match self {
            Self::Empty => {
                if !nodes.is_empty() {
                    anyhow::bail!("Expected no children, got {}", nodes.len());
                }

                Ok(())
            }
            Self::Single(expected) => {
                if nodes.len() != 1 {
                    anyhow::bail!("Expected 1 child, got {}", nodes.len());
                }

                if nodes[0] != *expected {
                    anyhow::bail!("Expected {:?}, got {:?}", expected, nodes[0]);
                }

                Ok(())
            }
            Self::Choice(choices) => {
                if nodes.len() != 1 {
                    anyhow::bail!("Choice requires exactly 1 child, got {}", nodes.len());
                }

                let node = nodes[0];
                for choice in choices {
                    if choice.matches(node) {
                        return Ok(());
                    }
                }

                anyhow::bail!(
                    "Node {:?} doesn't match any of the allowed types in choice",
                    node
                );
            }
            Self::ZeroOrMore(expr) => {
                for &node in nodes {
                    if !expr.matches(node) {
                        anyhow::bail!(
                            "Node {:?} doesn't match the allowed type in zero-or-more",
                            node
                        );
                    }
                }

                Ok(())
            }
            Self::OneOrMore(expr) => {
                if nodes.is_empty() {
                    anyhow::bail!("OneOrMore requires at least 1 child");
                }

                for &node in nodes {
                    if !expr.matches(node) {
                        anyhow::bail!(
                            "Node {:?} doesn't match the allowed type in one-or-more",
                            node
                        );
                    }
                }

                Ok(())
            }
            Self::Optional(expr) => {
                if nodes.len() > 1 {
                    anyhow::bail!("Optional allows at most 1 child, got {}", nodes.len());
                }

                if nodes.len() == 1 && !expr.matches(nodes[0]) {
                    anyhow::bail!(
                        "Node {:?} doesn't match the allowed type in optional",
                        nodes[0]
                    );
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
                                anyhow::bail!("OneOrMore requires at least one matching node");
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
                                anyhow::bail!("Expected {:?}, but no more children", expected);
                            }

                            if nodes[node_idx] != *expected {
                                anyhow::bail!("Expected {:?}, got {:?}", expected, nodes[node_idx]);
                            }

                            node_idx += 1;
                        }
                        _ => {
                            unimplemented!()
                        }
                    }
                }

                if node_idx < nodes.len() {
                    anyhow::bail!(
                        "Unexpected extra children: {} nodes remaining",
                        nodes.len() - node_idx
                    );
                }

                Ok(())
            }
        }
    }

    pub fn first_allowed_type(&self) -> Option<NodeType> {
        match self {
            Self::Empty => None,
            Self::Single(t) => Some(*t),
            Self::Choice(choices) => choices.first().and_then(|c| c.first_allowed_type()),
            Self::ZeroOrMore(_) | Self::Optional(_) => None,
            Self::OneOrMore(inner) => inner.first_allowed_type(),
            Self::Seq(exprs) => exprs.first().and_then(|e| e.first_allowed_type()),
        }
    }

    pub fn repair(&self, children: &[NodeType]) -> Vec<RepairAction> {
        let mut actions = vec![];

        for (i, child) in children.iter().enumerate().rev() {
            if !self.matches(*child) {
                actions.push(RepairAction::Remove { index: i });
            }
        }

        let valid_children: Vec<_> = children
            .iter()
            .enumerate()
            .filter(|(_, c)| self.matches(**c))
            .map(|(i, c)| (i, *c))
            .collect();
        let valid_types: Vec<_> = valid_children.iter().map(|(_, t)| *t).collect();

        actions.extend(self.compute_inserts(&valid_types));

        actions
    }

    fn compute_inserts(&self, children: &[NodeType]) -> Vec<RepairAction> {
        match self {
            Self::Empty => vec![],
            Self::Single(expected) => {
                if children.is_empty() || !children.contains(expected) {
                    vec![RepairAction::Insert {
                        index: children.len(),
                        node_type: *expected,
                    }]
                } else {
                    vec![]
                }
            }
            Self::Choice(choices) => {
                if children.is_empty() {
                    if let Some(t) = choices.first().and_then(|c| c.first_allowed_type()) {
                        vec![RepairAction::Insert {
                            index: 0,
                            node_type: t,
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            Self::ZeroOrMore(_) | Self::Optional(_) => vec![],
            Self::OneOrMore(inner) => {
                if children.is_empty() || !children.iter().any(|c| inner.matches(*c)) {
                    if let Some(t) = inner.first_allowed_type() {
                        vec![RepairAction::Insert {
                            index: 0,
                            node_type: t,
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            Self::Seq(exprs) => self.compute_seq_inserts(exprs, children),
        }
    }

    fn compute_seq_inserts(
        &self,
        exprs: &[ContentExpr],
        children: &[NodeType],
    ) -> Vec<RepairAction> {
        let mut actions = vec![];
        let mut child_idx = 0;
        let mut insert_offset = 0;

        for (expr_idx, expr) in exprs.iter().enumerate() {
            let required_after: usize =
                exprs[expr_idx + 1..].iter().map(|e| e.min_required()).sum();
            let remaining_children = children.len() - child_idx;
            let max_consume = remaining_children.saturating_sub(required_after);

            match expr {
                ContentExpr::ZeroOrMore(inner) | ContentExpr::Optional(inner) => {
                    let mut consumed = 0;
                    while consumed < max_consume
                        && child_idx < children.len()
                        && inner.matches(children[child_idx])
                    {
                        child_idx += 1;
                        consumed += 1;
                    }
                }
                ContentExpr::OneOrMore(inner) => {
                    let start = child_idx;
                    let mut consumed = 0;
                    while child_idx < children.len() && inner.matches(children[child_idx]) {
                        if consumed > 0 && consumed >= max_consume {
                            break;
                        }
                        child_idx += 1;
                        consumed += 1;
                    }
                    if child_idx == start {
                        if let Some(t) = inner.first_allowed_type() {
                            actions.push(RepairAction::Insert {
                                index: child_idx + insert_offset,
                                node_type: t,
                            });
                            insert_offset += 1;
                        }
                    }
                }
                ContentExpr::Single(expected) => {
                    if child_idx < children.len() && children[child_idx] == *expected {
                        child_idx += 1;
                    } else {
                        let current_valid_for_prev = child_idx < children.len()
                            && exprs[..expr_idx]
                                .iter()
                                .any(|prev| prev.matches(children[child_idx]));

                        let insert_index = if current_valid_for_prev {
                            children.len() + insert_offset
                        } else {
                            child_idx + insert_offset
                        };

                        actions.push(RepairAction::Insert {
                            index: insert_index,
                            node_type: *expected,
                        });
                        insert_offset += 1;
                    }
                }
                ContentExpr::Choice(choices) => {
                    if child_idx < children.len()
                        && choices.iter().any(|c| c.matches(children[child_idx]))
                    {
                        child_idx += 1;
                    } else {
                        let current_valid_for_prev = child_idx < children.len()
                            && exprs[..expr_idx]
                                .iter()
                                .any(|prev| prev.matches(children[child_idx]));

                        let insert_index = if current_valid_for_prev {
                            children.len() + insert_offset
                        } else {
                            child_idx + insert_offset
                        };

                        if let Some(t) = choices.first().and_then(|c| c.first_allowed_type()) {
                            actions.push(RepairAction::Insert {
                                index: insert_index,
                                node_type: t,
                            });
                            insert_offset += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        actions
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepairAction {
    Remove { index: usize },
    Insert { index: usize, node_type: NodeType },
}
