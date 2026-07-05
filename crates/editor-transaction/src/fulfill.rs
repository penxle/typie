use editor_model::{ChildView, ContentExpr, NodeType, NodeView, Subtree};

use crate::Step;

/// Analyzes a node's content expression and returns InsertSubtree steps
/// needed to make it valid. Returns empty vec if already valid.
pub fn fulfill(node: &NodeView) -> Vec<Step> {
    let spec = node.spec();
    let child_types: Vec<NodeType> = child_types(node);

    if spec.content.matches_sequence(&child_types) {
        return vec![];
    }

    let insertions = compute_insertions(&spec.content, &child_types);
    insertions
        .into_iter()
        .map(|(index, node_type)| {
            let subtree = scaffold(node_type);
            Step::InsertSubtree {
                parent: node.id(),
                index,
                subtree,
            }
        })
        .collect()
}

fn child_types(node: &NodeView) -> Vec<NodeType> {
    node.children()
        .map(|c| match c {
            ChildView::Block(b) => b.node_type(),
            ChildView::Leaf(l) => l.node_type(),
        })
        .collect()
}

fn compute_insertions(content: &ContentExpr, existing: &[NodeType]) -> Vec<(usize, NodeType)> {
    match content {
        ContentExpr::Empty | ContentExpr::ZeroOrMore(_) | ContentExpr::Optional(_) => vec![],

        ContentExpr::Single(t) => {
            if existing.is_empty() {
                vec![(0, *t)]
            } else {
                vec![]
            }
        }

        ContentExpr::OneOrMore(inner) => {
            if existing.is_empty() {
                vec![(0, first_type(inner))]
            } else {
                vec![]
            }
        }

        ContentExpr::Choice(choices) => {
            if existing.is_empty() {
                vec![(0, first_type(&choices[0]))]
            } else {
                vec![]
            }
        }

        ContentExpr::Seq(exprs) => compute_seq_insertions(exprs, existing),
    }
}

/// Handle Seq patterns. Walk through expressions and existing children in parallel.
fn compute_seq_insertions(exprs: &[ContentExpr], existing: &[NodeType]) -> Vec<(usize, NodeType)> {
    let mut insertions = Vec::new();
    let mut existing_idx = 0;

    for expr in exprs.iter() {
        match expr {
            ContentExpr::Single(t) => {
                if existing_idx < existing.len() && existing[existing_idx] == *t {
                    existing_idx += 1;
                } else {
                    insertions.push((existing_idx + insertions.len(), *t));
                }
            }
            ContentExpr::ZeroOrMore(inner) | ContentExpr::OneOrMore(inner) => {
                let is_one_or_more = matches!(expr, ContentExpr::OneOrMore(_));

                let mut consumed = 0;
                while existing_idx < existing.len() && inner.matches(existing[existing_idx]) {
                    existing_idx += 1;
                    consumed += 1;
                }

                if is_one_or_more && consumed == 0 {
                    insertions.push((existing_idx + insertions.len(), first_type(inner)));
                }
            }
            ContentExpr::Optional(inner)
                if existing_idx < existing.len() && inner.matches(existing[existing_idx]) =>
            {
                existing_idx += 1;
            }
            ContentExpr::Optional(_) => {}
            _ => {}
        }
    }

    insertions
}

fn first_type(expr: &ContentExpr) -> NodeType {
    match expr {
        ContentExpr::Single(t) => *t,
        ContentExpr::Choice(choices) => first_type(&choices[0]),
        ContentExpr::OneOrMore(inner)
        | ContentExpr::ZeroOrMore(inner)
        | ContentExpr::Optional(inner) => first_type(inner),
        ContentExpr::Seq(exprs) => first_type(&exprs[0]),
        ContentExpr::Empty => unreachable!("Empty content has no type"),
    }
}

/// Build minimum valid subtree for a NodeType, recursively filling required children.
fn scaffold(node_type: NodeType) -> Subtree {
    let node = node_type.into_node().to_plain();
    let spec = node_type.spec();
    let children = scaffold_children(&spec.content);

    Subtree {
        node,
        modifiers: vec![],
        marker: None,
        children,
    }
}

fn scaffold_children(content: &ContentExpr) -> Vec<Subtree> {
    match content {
        ContentExpr::Empty | ContentExpr::ZeroOrMore(_) | ContentExpr::Optional(_) => vec![],
        ContentExpr::Single(t) => vec![scaffold(*t)],
        ContentExpr::OneOrMore(inner) => vec![scaffold(first_type(inner))],
        ContentExpr::Choice(choices) => vec![scaffold(first_type(&choices[0]))],
        ContentExpr::Seq(exprs) => exprs.iter().flat_map(scaffold_children).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    // The projected DocView is always normalized (missing required children are
    // synthesized as derived nodes), so `fulfill` observes only already-valid
    // nodes here and returns no repair steps. Repair-step generation against
    // partially-built structures is exercised through the command layer (M2).
    #[test]
    fn fulfill_valid_root_returns_empty() {
        let (state, ..) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let view = state.view();
        let root = view.root().unwrap();
        assert!(fulfill(&root).is_empty());
    }
}
