use std::collections::VecDeque;

use crate::NodeType;

use super::{ContentExpr, ContextExpr, Schema};

pub fn context_allows(path: &[NodeType], t: NodeType) -> bool {
    let ctx = &Schema::node_spec(t).context;
    if *ctx == ContextExpr::Any {
        return true;
    }
    let full: Vec<NodeType> = path.iter().copied().chain(std::iter::once(t)).collect();
    ctx.matches(&full)
}

// Declared content-expr order, not enum order: the chain tie-break is defined by
// declaration order, which differs from NodeType's enum order (e.g. Root content).
fn accepted_in_declared_order(parent: NodeType) -> Vec<NodeType> {
    fn walk(e: &ContentExpr, out: &mut Vec<NodeType>) {
        match e {
            ContentExpr::Empty | ContentExpr::Any => {}
            ContentExpr::Single(t) => {
                if !out.contains(t)
                    && !matches!(t, NodeType::Unknown)
                    && !Schema::node_spec(*t).inline
                {
                    out.push(*t);
                }
            }
            ContentExpr::Seq(es) | ContentExpr::Choice(es) => es.iter().for_each(|e| walk(e, out)),
            ContentExpr::ZeroOrMore(e) | ContentExpr::OneOrMore(e) | ContentExpr::Optional(e) => {
                walk(e, out)
            }
        }
    }
    let mut out = Vec::new();
    walk(&Schema::node_spec(parent).content, &mut out);
    out
}

pub fn wrap_chain(path: &[NodeType], child: NodeType) -> Option<Vec<NodeType>> {
    let parent = *path.last().expect("path includes parent");
    if Schema::node_spec(parent).content.matches(child) && context_allows(path, child) {
        return Some(vec![]);
    }
    let mut queue: VecDeque<Vec<NodeType>> = VecDeque::from([vec![]]);
    while let Some(chain) = queue.pop_front() {
        if chain.len() >= 4 {
            continue;
        }
        let tip = chain.last().copied().unwrap_or(parent);
        for w in accepted_in_declared_order(tip) {
            if chain.contains(&w) {
                continue;
            }
            let mut p: Vec<NodeType> = path.to_vec();
            p.extend_from_slice(&chain);
            if !context_allows(&p, w) {
                continue;
            }
            let mut next = chain.clone();
            next.push(w);
            p.push(w);
            if Schema::node_spec(w).content.matches(child) && context_allows(&p, child) {
                return Some(next);
            }
            queue.push_back(next);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeType;
    use strum::IntoEnumIterator;

    #[test]
    fn wrap_chain_derives_from_schema() {
        let root = &[NodeType::Root][..];
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::Paragraph], NodeType::Text),
            Some(vec![])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Text),
            Some(vec![NodeType::ListItem, NodeType::Paragraph])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Paragraph),
            Some(vec![NodeType::ListItem])
        );
        assert_eq!(
            wrap_chain(root, NodeType::TableCell),
            Some(vec![NodeType::Table, NodeType::TableRow])
        );
        assert_eq!(
            wrap_chain(root, NodeType::FoldTitle),
            Some(vec![NodeType::Fold])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Fold),
            None
        );
        assert_eq!(
            wrap_chain(
                &[
                    NodeType::Root,
                    NodeType::Table,
                    NodeType::TableRow,
                    NodeType::TableCell
                ],
                NodeType::TableRow
            ),
            None
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::Blockquote], NodeType::PageBreak),
            None
        );
        assert_eq!(
            wrap_chain(root, NodeType::PageBreak),
            Some(vec![NodeType::Paragraph])
        );
    }

    #[test]
    fn every_node_type_is_placeable_from_root() {
        for t in NodeType::iter() {
            if matches!(t, NodeType::Root | NodeType::Unknown) {
                continue;
            }
            assert!(
                wrap_chain(&[NodeType::Root], t).is_some(),
                "{t:?}는 Root로부터 context-유효 배치 체인이 없다 — NodeSpec 선언을 확인하라"
            );
        }
    }
}
