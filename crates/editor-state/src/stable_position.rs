use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{Doc, Node, NodeId};
use serde::{Deserialize, Serialize};

use crate::Position;
use crate::affinity::Affinity;
use crate::bind::Bind;

/// One link in the structural chain from root to the cursor's leaf node.
///
/// `child_dot` is this node's dot in its parent's `children` RGA. For the
/// root link, `child_dot` is unused (freeze writes `Dot::new(0, 0)`, thaw
/// ignores).
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChainLink {
    pub node_id: NodeId,
    pub child_dot: Dot,
}

/// A CRDT-dot-anchored position. The chain is always root-to-leaf inclusive;
/// `chain.last().node_id` is the host of the binding (text node for `Char`,
/// container for `Child` and `ContainerStart`).
#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StablePosition {
    Char {
        chain: Vec<ChainLink>,
        char_dot: Dot,
        bind: Bind,
        affinity: Affinity,
    },
    Child {
        chain: Vec<ChainLink>,
        child_dot: Dot,
        bind: Bind,
        affinity: Affinity,
    },
    ContainerStart {
        chain: Vec<ChainLink>,
        affinity: Affinity,
    },
}

impl StablePosition {
    pub fn chain(&self) -> &[ChainLink] {
        match self {
            Self::Char { chain, .. }
            | Self::Child { chain, .. }
            | Self::ContainerStart { chain, .. } => chain,
        }
    }

    pub fn affinity(&self) -> Affinity {
        match self {
            Self::Char { affinity, .. }
            | Self::Child { affinity, .. }
            | Self::ContainerStart { affinity, .. } => *affinity,
        }
    }

    /// Returns `Bind::Right` for `ContainerStart` (irrelevant; resolution is
    /// unconditional offset 0).
    pub fn bind(&self) -> Bind {
        match self {
            Self::Char { bind, .. } | Self::Child { bind, .. } => *bind,
            Self::ContainerStart { .. } => Bind::Right,
        }
    }
}

pub(crate) fn freeze_position(pos: Position, doc: &Doc) -> StablePosition {
    let entry = doc
        .get_entry(pos.node_id)
        .expect("freeze_position: pos must resolve against doc at freeze time");
    let chain = build_chain(pos.node_id, doc);

    match &entry.node {
        Node::Text(text) => {
            if text.text.is_empty() || pos.offset == 0 {
                StablePosition::ContainerStart {
                    chain,
                    affinity: pos.affinity,
                }
            } else {
                let char_dot = text
                    .text
                    .dot_at(pos.offset)
                    .expect("freeze_position: offset within text bounds")
                    .expect("freeze_position: dot_at within bounds yields Some");
                StablePosition::Char {
                    chain,
                    char_dot,
                    bind: Bind::Right,
                    affinity: pos.affinity,
                }
            }
        }
        _ => {
            let children = &entry.children;
            if children.is_empty() || pos.offset == 0 {
                StablePosition::ContainerStart {
                    chain,
                    affinity: pos.affinity,
                }
            } else {
                let child_dot = children
                    .dot_at(pos.offset)
                    .expect("freeze_position: offset within children bounds")
                    .expect("freeze_position: dot_at within bounds yields Some");
                StablePosition::Child {
                    chain,
                    child_dot,
                    bind: Bind::Right,
                    affinity: pos.affinity,
                }
            }
        }
    }
}

pub(crate) fn thaw_position(sp: &StablePosition, doc: &Doc) -> Position {
    let chain = sp.chain();
    let mut live_idx: usize = 0;
    for (i, link) in chain.iter().enumerate() {
        if doc.get_entry(link.node_id).is_some() {
            live_idx = i;
        } else {
            break;
        }
    }

    if live_idx == chain.len() - 1 {
        return resolve_primary(sp, doc);
    }

    let anc_id = chain[live_idx].node_id;
    let anc = doc.get_entry(anc_id).expect("live ancestor must exist");
    let dead_dot = chain[live_idx + 1].child_dot;
    let offset = nearest_live_sibling_offset(&anc.children, dead_dot, sp.bind());
    Position {
        node_id: anc_id,
        offset,
        affinity: sp.affinity(),
    }
}

fn resolve_primary(sp: &StablePosition, doc: &Doc) -> Position {
    let leaf_id = sp.chain().last().expect("non-empty chain").node_id;
    let entry = doc.get_entry(leaf_id).expect("leaf alive");
    match sp {
        StablePosition::Char {
            char_dot,
            bind,
            affinity,
            ..
        } => match &entry.node {
            Node::Text(text) => {
                resolve_dot_in_text(&text.text, *char_dot, *bind, leaf_id, *affinity)
            }
            _ => Position {
                node_id: leaf_id,
                offset: 0,
                affinity: *affinity,
            },
        },
        StablePosition::Child {
            child_dot,
            bind,
            affinity,
            ..
        } => match &entry.node {
            Node::Text(_) => Position {
                node_id: leaf_id,
                offset: 0,
                affinity: *affinity,
            },
            _ => resolve_dot_in_children(&entry.children, *child_dot, *bind, leaf_id, *affinity),
        },
        StablePosition::ContainerStart { affinity, .. } => Position {
            node_id: leaf_id,
            offset: 0,
            affinity: *affinity,
        },
    }
}

fn resolve_dot_in_text(
    text: &editor_crdt::Text,
    dot: editor_crdt::Dot,
    bind: Bind,
    node_id: NodeId,
    aff: Affinity,
) -> Position {
    if !text.contains_dot(dot) {
        return Position {
            node_id,
            offset: 0,
            affinity: aff,
        };
    }
    match text.live_offset_of(dot) {
        Some(off) => {
            let offset = match bind {
                Bind::Left => off,
                Bind::Right => off + 1,
            };
            Position {
                node_id,
                offset,
                affinity: aff,
            }
        }
        None => {
            let offset = match bind {
                Bind::Right => text.next_live_offset_after(dot).unwrap_or(text.len()),
                Bind::Left => text
                    .prev_live_offset_before(dot)
                    .map(|o| o + 1)
                    .unwrap_or(0),
            };
            Position {
                node_id,
                offset,
                affinity: aff,
            }
        }
    }
}

fn resolve_dot_in_children(
    children: &editor_crdt::Rga<NodeId>,
    dot: editor_crdt::Dot,
    bind: Bind,
    node_id: NodeId,
    aff: Affinity,
) -> Position {
    if !children.contains_dot(dot) {
        return Position {
            node_id,
            offset: 0,
            affinity: aff,
        };
    }
    match children.live_offset_of(dot) {
        Some(off) => {
            let offset = match bind {
                Bind::Left => off,
                Bind::Right => off + 1,
            };
            Position {
                node_id,
                offset,
                affinity: aff,
            }
        }
        None => {
            let offset = match bind {
                Bind::Right => children
                    .next_live_offset_after(dot)
                    .unwrap_or(children.len()),
                Bind::Left => children
                    .prev_live_offset_before(dot)
                    .map(|o| o + 1)
                    .unwrap_or(0),
            };
            Position {
                node_id,
                offset,
                affinity: aff,
            }
        }
    }
}

fn nearest_live_sibling_offset(
    children: &editor_crdt::Rga<NodeId>,
    dead_dot: editor_crdt::Dot,
    bind: Bind,
) -> usize {
    if !children.contains_dot(dead_dot) {
        return 0;
    }
    match bind {
        Bind::Right => children
            .next_live_offset_after(dead_dot)
            .unwrap_or(children.len()),
        Bind::Left => children
            .prev_live_offset_before(dead_dot)
            .map(|o| o + 1)
            .unwrap_or(0),
    }
}

pub(crate) fn build_chain(node_id: NodeId, doc: &Doc) -> Vec<ChainLink> {
    let mut chain: Vec<ChainLink> = Vec::new();
    let mut cur = Some(node_id);
    while let Some(id) = cur {
        let entry = doc
            .get_entry(id)
            .expect("build_chain: ancestor must be alive in doc at freeze time");
        let parent = *entry.parent.get();
        let child_dot = match parent {
            Some(parent_id) => {
                let parent_entry = doc
                    .get_entry(parent_id)
                    .expect("build_chain: parent must be alive in doc at freeze time");
                parent_entry
                    .children
                    .dot_for(&id)
                    .expect("build_chain: child must be present in parent's children RGA")
            }
            None => Dot::new(0, 0),
        };
        chain.push(ChainLink {
            node_id: id,
            child_dot,
        });
        cur = parent;
    }
    chain.reverse();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::Dot;
    use editor_model::NodeId;

    #[test]
    fn build_chain_walks_root_to_leaf_inclusive() {
        use editor_macros::doc;

        let (doc, p1, t1) = doc! {
            root {
                p1: paragraph {
                    t1: text("hi")
                }
            }
        };
        let chain = super::build_chain(t1, &doc);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].node_id, NodeId::ROOT);
        assert_eq!(chain[1].node_id, p1);
        assert_eq!(chain[2].node_id, t1);
        assert_eq!(chain[0].child_dot, Dot::new(0, 0));
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(chain[1].child_dot, root.children.dot_for(&p1).unwrap());
        let p1_entry = doc.get_entry(p1).unwrap();
        assert_eq!(chain[2].child_dot, p1_entry.children.dot_for(&t1).unwrap());
    }

    #[test]
    fn accessors_return_variant_fields() {
        let chain = vec![ChainLink {
            node_id: NodeId::ROOT,
            child_dot: Dot::new(0, 0),
        }];
        let sp = StablePosition::Char {
            chain: chain.clone(),
            char_dot: Dot::new(1, 0),
            bind: Bind::Right,
            affinity: Affinity::default(),
        };
        assert_eq!(sp.chain(), chain.as_slice());
        assert_eq!(sp.bind(), Bind::Right);
    }

    #[test]
    fn freeze_offset_zero_text_yields_container_start() {
        use editor_macros::doc;
        let (doc, t1) = doc! { root { paragraph { t1: text("hi") } } };
        let pos = crate::Position::new(t1, 0);
        let sp = super::freeze_position(pos, &doc);
        assert!(matches!(sp, StablePosition::ContainerStart { .. }));
        assert_eq!(sp.chain().last().unwrap().node_id, t1);
    }

    #[test]
    fn freeze_offset_one_text_yields_char_right_on_first_char() {
        use editor_macros::doc;
        let (doc, t1) = doc! { root { paragraph { t1: text("hi") } } };
        let pos = crate::Position::new(t1, 1);
        let sp = super::freeze_position(pos, &doc);
        match sp {
            StablePosition::Char {
                chain,
                char_dot,
                bind,
                ..
            } => {
                assert_eq!(chain.last().unwrap().node_id, t1);
                assert_eq!(bind, Bind::Right);
                let entry = doc.get_entry(t1).unwrap();
                let text = match &entry.node {
                    editor_model::Node::Text(t) => t,
                    _ => panic!("t1 must be a Text node"),
                };
                assert_eq!(char_dot, text.text.dot_at(1).unwrap().unwrap());
            }
            other => panic!("expected Char, got {:?}", other),
        }
    }

    #[test]
    fn freeze_empty_text_yields_container_start() {
        use editor_macros::doc;
        let (doc, t1) = doc! { root { paragraph { t1: text("") } } };
        let pos = crate::Position::new(t1, 0);
        let sp = super::freeze_position(pos, &doc);
        assert!(matches!(sp, StablePosition::ContainerStart { .. }));
    }

    #[test]
    fn freeze_container_offset_zero_yields_container_start() {
        use editor_macros::doc;
        let (doc, p1) = doc! { root { p1: paragraph { text("x") } } };
        let pos = crate::Position::new(p1, 0);
        let sp = super::freeze_position(pos, &doc);
        assert!(matches!(sp, StablePosition::ContainerStart { .. }));
        assert_eq!(sp.chain().last().unwrap().node_id, p1);
    }

    #[test]
    fn freeze_container_middle_yields_child_right() {
        use editor_macros::doc;
        let (doc, p1, t1) = doc! { root { p1: paragraph { t1: text("x") } } };
        let pos = crate::Position::new(p1, 1);
        let sp = super::freeze_position(pos, &doc);
        match sp {
            StablePosition::Child {
                chain,
                child_dot,
                bind,
                ..
            } => {
                assert_eq!(chain.last().unwrap().node_id, p1);
                assert_eq!(bind, Bind::Right);
                let p1_entry = doc.get_entry(p1).unwrap();
                assert_eq!(child_dot, p1_entry.children.dot_for(&t1).unwrap());
            }
            other => panic!("expected Child, got {:?}", other),
        }
    }

    #[test]
    fn thaw_roundtrip_char_middle_in_unchanged_doc() {
        use editor_macros::doc;
        let (doc, t1) = doc! { root { paragraph { t1: text("hello") } } };
        let pos = crate::Position::new(t1, 3);
        let sp = super::freeze_position(pos, &doc);
        let back = super::thaw_position(&sp, &doc);
        assert_eq!(back, pos);
    }

    #[test]
    fn thaw_roundtrip_container_start() {
        use editor_macros::doc;
        let (doc, t1) = doc! { root { paragraph { t1: text("hi") } } };
        let pos = crate::Position::new(t1, 0);
        let sp = super::freeze_position(pos, &doc);
        let back = super::thaw_position(&sp, &doc);
        assert_eq!(back, pos);
    }

    #[test]
    fn thaw_char_dot_tombstoned_walks_to_nearest_live_in_bind_direction() {
        use editor_macros::state;
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("abc") } } }
            selection: (t1, 0)
        };
        let pos = crate::Position::new(t1, 2);
        let sp = super::freeze_position(pos, &state.doc);

        // Rga::dot_at(N) yields the entry at iter position N-1; offset 0 is the
        // "before first" sentinel returning None. dot_at(2) targets 'b'.
        let b_dot = {
            let entry = state.doc.get_entry(t1).unwrap();
            let text = match &entry.node {
                editor_model::Node::Text(t) => t,
                _ => unreachable!(),
            };
            text.text.dot_at(2).unwrap().unwrap()
        };
        let (state, _op) = state
            .apply(editor_model::DocOp::Text {
                node_id: t1,
                op: editor_crdt::TextOp::RemoveChar { observed: b_dot },
            })
            .unwrap();

        let back = super::thaw_position(&sp, &state.doc);
        assert_eq!(back.node_id, t1);
        assert_eq!(back.offset, 1);
    }

    #[test]
    fn thaw_resurrection_dot_absent_node_alive_returns_offset_zero() {
        // `Doc::from_plain` rebuilds the doc under a fresh actor seed, so replayed
        // text inserts get new Dots while NodeIds are preserved verbatim — the
        // exact shape needed to exercise the resurrection branch.
        use editor_macros::doc;
        let (doc1, t1) = doc! { root { paragraph { t1: text("a") } } };
        let pos = crate::Position::new(t1, 1);
        let sp = super::freeze_position(pos, &doc1);

        let plain = doc1.to_plain();
        let (doc2, _) = editor_model::Doc::from_plain(plain);

        let doc2_t1 = doc2.get_entry(t1).unwrap();
        let doc2_text = match &doc2_t1.node {
            editor_model::Node::Text(t) => t,
            _ => unreachable!(),
        };
        let sp_char_dot = match &sp {
            StablePosition::Char { char_dot, .. } => *char_dot,
            other => panic!("expected Char, got {:?}", other),
        };
        assert!(!doc2_text.text.contains_dot(sp_char_dot));

        let back = super::thaw_position(&sp, &doc2);
        assert_eq!(back.node_id, t1);
        assert_eq!(back.offset, 0);
    }

    #[test]
    fn thaw_leaf_dead_walks_to_nearest_live_sibling_right() {
        use editor_macros::state;
        let (state, p2, t2) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    p2: paragraph { t2: text("b") }
                    paragraph { text("c") }
                }
            }
            selection: (t2, 0)
        };
        let pos = crate::Position::new(t2, 0);
        let sp = super::freeze_position(pos, &state.doc);

        let dead_state = remove_node(&state, p2);

        let back = super::thaw_position(&sp, &dead_state.doc);
        assert_eq!(back.node_id, editor_model::NodeId::ROOT);
        assert_eq!(back.offset, 1);
    }

    fn remove_node(state: &crate::State, node_id: editor_model::NodeId) -> crate::State {
        use editor_crdt::{OrMapOp, RgaOp};
        use editor_model::DocOp;

        let root_id = editor_model::NodeId::ROOT;
        let node_dot = {
            let root_entry = state.doc.get_entry(root_id).unwrap();
            root_entry
                .children
                .iter_with_dot()
                .find(|(_, v)| **v == node_id)
                .map(|(d, _)| d)
                .expect("node must be a child of root")
        };

        let mut to_remove: Vec<editor_model::NodeId> = Vec::new();
        let mut stack = vec![node_id];
        while let Some(id) = stack.pop() {
            to_remove.push(id);
            let entry = state.doc.get_entry(id).expect("node alive at freeze time");
            for child in entry.children.iter() {
                stack.push(*child);
            }
        }

        let (next, _ops) = state
            .batch_with_ops::<_, crate::StateError>(|b| {
                // Tombstone descendants before unlinking the subtree root, or
                // Doc::verify's reachability check rejects the resulting doc.
                for id in to_remove.iter().rev() {
                    let presence_dots: Vec<_> = b.doc.nodes_tags_for(id).copied().collect();
                    b.apply(DocOp::Presence {
                        node_id: *id,
                        op: OrMapOp::Unset {
                            observed: presence_dots,
                        },
                    })?;
                }
                b.apply(DocOp::Children {
                    node_id: root_id,
                    op: RgaOp::Remove { observed: node_dot },
                })?;
                Ok(())
            })
            .unwrap();
        next
    }

    #[test]
    fn thaw_leaf_dead_no_live_siblings_clamps_to_zero() {
        use editor_macros::state;
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("x") } } }
            selection: (t1, 0)
        };
        let pos = crate::Position::new(t1, 0);
        let sp = super::freeze_position(pos, &state.doc);

        let dead_state = remove_node(&state, p1);

        let back = super::thaw_position(&sp, &dead_state.doc);
        assert_eq!(back.node_id, editor_model::NodeId::ROOT);
        assert_eq!(back.offset, 0);
    }
}
