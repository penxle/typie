use crate::model::{Doc, NodeId};

fn ancestors_including_self(doc: &Doc, node_id: NodeId) -> Option<Vec<NodeId>> {
    let node = doc.node(node_id)?;
    Some(
        node.ancestors()
            .map(|ancestor| ancestor.node_id())
            .collect(),
    )
}

pub fn lowest_common_ancestor_id(doc: &Doc, a: NodeId, b: NodeId) -> Option<NodeId> {
    let ancestors_a = ancestors_including_self(doc, a)?;
    let ancestors_b = ancestors_including_self(doc, b)?;

    let mut lca = None;
    for (lhs, rhs) in ancestors_a.iter().rev().zip(ancestors_b.iter().rev()) {
        if lhs == rhs {
            lca = Some(*lhs);
        } else {
            break;
        }
    }

    lca
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowest_common_ancestor_for_siblings_is_parent() {
        let mut callout = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let doc = doc! {
            @callout callout {
                @p1 paragraph { text { "a" } }
                @p2 paragraph { text { "b" } }
            }
        };

        let lca = lowest_common_ancestor_id(&doc, p1, p2);
        assert_eq!(lca, Some(callout));
    }

    #[test]
    fn lowest_common_ancestor_inside_and_outside_is_root() {
        let mut outside = id!();
        let mut inside = id!();

        let doc = doc! {
            @outside paragraph { text { "outside" } }
            callout {
                @inside paragraph { text { "inside" } }
            }
        };

        let lca = lowest_common_ancestor_id(&doc, outside, inside);
        assert_eq!(lca, Some(NodeId::ROOT));
    }

    #[test]
    fn lowest_common_ancestor_is_none_when_node_missing() {
        let mut existing = id!();
        let missing = NodeId::new();

        let doc = doc! {
            @existing paragraph { text { "a" } }
        };

        let lca = lowest_common_ancestor_id(&doc, existing, missing);
        assert_eq!(lca, None);
    }
}
