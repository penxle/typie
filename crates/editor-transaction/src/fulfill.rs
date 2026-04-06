use editor_model::*;
use editor_schema::{ContentExpr, NodeSpecExt};

use crate::Step;

/// Analyzes a node's content expression and returns InsertSubtree steps
/// needed to make it valid. Returns empty vec if already valid.
pub fn fulfill(node: &NodeRef) -> Vec<Step> {
    let spec = node.spec();
    let child_types: Vec<NodeType> = node.children().map(|c| c.as_type()).collect();

    if spec.content.matches_sequence(&child_types) {
        return vec![];
    }

    let insertions = compute_insertions(&spec.content, &child_types);
    insertions
        .into_iter()
        .map(|(index, node_type)| {
            let subtree = scaffold(node_type);
            Step::InsertSubtree {
                parent_id: node.id(),
                index,
                subtree,
            }
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
    let id = NodeId::new();
    let node = node_type.into_node();
    let spec = node_type.spec();
    let children = scaffold_children(&spec.content);

    Subtree {
        id,
        node,
        modifiers: vec![],
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
    use editor_macros::doc;

    use super::*;

    #[test]
    fn fulfill_valid_node_returns_empty() {
        // Root with Paragraph child is valid: (choice)*, Paragraph
        let (doc, ..) = doc! {
            root {
                paragraph
            }
        };

        let root = doc.node(NodeId::ROOT).unwrap();
        let steps = fulfill(&root);
        assert!(steps.is_empty());
    }

    #[test]
    fn fulfill_root_missing_trailing_paragraph() {
        // Root with only Blockquote -> missing trailing Paragraph
        let (doc, ..) = doc! {
            root {
                blockquote {
                    paragraph
                }
            }
        };

        let root = doc.node(NodeId::ROOT).unwrap();
        let steps = fulfill(&root);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, NodeId::ROOT);
                assert_eq!(*index, 1);
                assert!(matches!(subtree.node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn fulfill_empty_blockquote_inserts_paragraph() {
        // Blockquote with no children -> needs (P|BL|OL)+ -> insert Paragraph
        let (doc, bq1, ..) = doc! {
            root {
                bq1: blockquote
            }
        };

        let bq = doc.node(bq1).unwrap();
        let steps = fulfill(&bq);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree { subtree, .. } => {
                assert!(matches!(subtree.node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn fulfill_empty_bullet_list_inserts_list_item_with_paragraph() {
        // BulletList with no children -> needs ListItem+ -> insert ListItem(Paragraph)
        let (doc, bl1, ..) = doc! {
            root {
                bl1: bullet_list
            }
        };

        let list = doc.node(bl1).unwrap();
        let steps = fulfill(&list);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree { subtree, .. } => {
                assert!(matches!(subtree.node, Node::ListItem(_)));
                assert_eq!(subtree.children.len(), 1);
                assert!(matches!(subtree.children[0].node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn scaffold_produces_minimum_valid_subtree() {
        let tree = scaffold(NodeType::BulletList);
        assert!(matches!(tree.node, Node::BulletList(_)));
        assert_eq!(tree.children.len(), 1);

        let item = &tree.children[0];
        assert!(matches!(item.node, Node::ListItem(_)));
        assert_eq!(item.children.len(), 1);

        let para = &item.children[0];
        assert!(matches!(para.node, Node::Paragraph(_)));
        assert!(para.children.is_empty());
    }
}
