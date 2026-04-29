use std::collections::{HashMap, HashSet};

use editor_model::{Doc, NodeId};

use crate::sync::conflict::{
    BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeAction {
    Unchanged,
    Moved { to_parent: NodeId },
    Edited,
    Deleted,
    Added,
    // Not in classify_actions output — fallback for "NodeId from the other side's actions only"
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeDecision {
    Keep,
    Delete,
    MoveTo { parent: NodeId },
    Add,
}

#[derive(Debug, Clone)]
pub struct TreeMergeResult {
    pub decisions: HashMap<NodeId, NodeDecision>,
}

fn node_parent_in(doc: &Doc, id: NodeId) -> Option<NodeId> {
    doc.nodes.get(&id).and_then(|e| e.parent)
}

fn classify_actions(base: &Doc, candidate: &Doc) -> HashMap<NodeId, NodeAction> {
    let mut out = HashMap::new();

    for (id, base_entry) in base.nodes.iter() {
        match candidate.nodes.get(id) {
            None => {
                out.insert(*id, NodeAction::Deleted);
            }
            Some(cand_entry) => {
                if base_entry.parent != cand_entry.parent {
                    out.insert(
                        *id,
                        NodeAction::Moved {
                            // Only the root has parent: None; any other node in candidate must have
                            // Some(parent). Root's parent cannot change, so the fallback is never
                            // reached in practice.
                            to_parent: cand_entry.parent.unwrap_or(NodeId::ROOT),
                        },
                    );
                } else if base_entry.node != cand_entry.node
                    || base_entry.modifiers != cand_entry.modifiers
                {
                    out.insert(*id, NodeAction::Edited);
                } else {
                    out.insert(*id, NodeAction::Unchanged);
                }
            }
        }
    }

    for id in candidate.nodes.keys() {
        if !base.nodes.contains_key(id) {
            out.insert(*id, NodeAction::Added);
        }
    }

    out
}

pub fn merge_tree(base: &Doc, ours: &Doc, theirs: &Doc) -> (TreeMergeResult, Vec<ConflictRecord>) {
    let ours_actions = classify_actions(base, ours);
    let theirs_actions = classify_actions(base, theirs);

    let all_ids: HashSet<NodeId> = ours_actions
        .keys()
        .chain(theirs_actions.keys())
        .copied()
        .collect();

    let mut decisions: HashMap<NodeId, NodeDecision> = HashMap::new();
    let mut conflicts: Vec<ConflictRecord> = Vec::new();

    for node_id in all_ids {
        let o = ours_actions
            .get(&node_id)
            .copied()
            .unwrap_or(NodeAction::Absent);
        let t = theirs_actions
            .get(&node_id)
            .copied()
            .unwrap_or(NodeAction::Absent);

        let decision = match (o, t) {
            (NodeAction::Unchanged, NodeAction::Unchanged) => NodeDecision::Keep,

            (NodeAction::Moved { to_parent }, NodeAction::Unchanged)
            | (NodeAction::Unchanged, NodeAction::Moved { to_parent }) => {
                NodeDecision::MoveTo { parent: to_parent }
            }

            (NodeAction::Moved { to_parent: po }, NodeAction::Moved { to_parent: pt })
                if po == pt =>
            {
                NodeDecision::MoveTo { parent: po }
            }

            (NodeAction::Moved { to_parent: po }, NodeAction::Moved { to_parent: pt }) => {
                conflicts.push(position_conflict(node_id, po, pt));
                NodeDecision::MoveTo { parent: po }
            }

            (NodeAction::Deleted, NodeAction::Unchanged)
            | (NodeAction::Unchanged, NodeAction::Deleted)
            | (NodeAction::Deleted, NodeAction::Deleted) => NodeDecision::Delete,

            (NodeAction::Deleted, NodeAction::Edited) => {
                conflicts.push(lifecycle_conflict(base, node_id, "deleted", "edited"));
                NodeDecision::Delete
            }
            (NodeAction::Deleted, NodeAction::Moved { .. }) => {
                conflicts.push(lifecycle_conflict(base, node_id, "deleted", "moved"));
                NodeDecision::Delete
            }
            (NodeAction::Edited, NodeAction::Deleted) => {
                conflicts.push(lifecycle_conflict(base, node_id, "edited", "deleted"));
                NodeDecision::Keep
            }
            (NodeAction::Moved { to_parent }, NodeAction::Deleted) => {
                conflicts.push(lifecycle_conflict(base, node_id, "moved", "deleted"));
                NodeDecision::MoveTo { parent: to_parent }
            }

            (NodeAction::Added, NodeAction::Absent) | (NodeAction::Absent, NodeAction::Added) => {
                NodeDecision::Add
            }

            (NodeAction::Added, NodeAction::Added) => NodeDecision::Add,

            (NodeAction::Edited, NodeAction::Edited)
            | (NodeAction::Edited, NodeAction::Unchanged)
            | (NodeAction::Unchanged, NodeAction::Edited) => NodeDecision::Keep,

            (NodeAction::Edited, NodeAction::Moved { to_parent })
            | (NodeAction::Moved { to_parent }, NodeAction::Edited) => {
                NodeDecision::MoveTo { parent: to_parent }
            }

            // Structurally impossible: Added means not-in-base, but Unchanged/Moved/Edited/Deleted
            // all require the node to be in base. These pairs cannot co-occur.
            // Similarly Absent means not in this side's action map at all, which only happens when
            // the node came solely from the other side's classify_actions — it cannot appear paired
            // with any in-base action from that side.
            (NodeAction::Added, NodeAction::Unchanged)
            | (NodeAction::Added, NodeAction::Moved { .. })
            | (NodeAction::Added, NodeAction::Edited)
            | (NodeAction::Added, NodeAction::Deleted)
            | (NodeAction::Unchanged, NodeAction::Added)
            | (NodeAction::Moved { .. }, NodeAction::Added)
            | (NodeAction::Edited, NodeAction::Added)
            | (NodeAction::Deleted, NodeAction::Added)
            | (NodeAction::Absent, NodeAction::Unchanged)
            | (NodeAction::Absent, NodeAction::Moved { .. })
            | (NodeAction::Absent, NodeAction::Edited)
            | (NodeAction::Absent, NodeAction::Deleted)
            | (NodeAction::Unchanged, NodeAction::Absent)
            | (NodeAction::Moved { .. }, NodeAction::Absent)
            | (NodeAction::Edited, NodeAction::Absent)
            | (NodeAction::Deleted, NodeAction::Absent)
            | (NodeAction::Absent, NodeAction::Absent) => unreachable!(
                "impossible action pair: {:?} vs {:?} for {:?}",
                o, t, node_id
            ),
        };

        decisions.insert(node_id, decision);
    }

    (TreeMergeResult { decisions }, conflicts)
}

fn position_conflict(
    node_id: NodeId,
    ours_parent: NodeId,
    theirs_parent: NodeId,
) -> ConflictRecord {
    ConflictRecord {
        kind: ConflictKind::Position,
        target: ConflictTarget::Position { node_id },
        base_value: None,
        branches: vec![
            ConflictBranch {
                side: BranchSide::Ours,
                value: serde_json::to_value(ours_parent).unwrap().into(),
            },
            ConflictBranch {
                side: BranchSide::Theirs,
                value: serde_json::to_value(theirs_parent).unwrap().into(),
            },
        ],
        auto_resolved: BranchSide::Ours,
    }
}

fn lifecycle_conflict(
    base: &Doc,
    node_id: NodeId,
    ours_label: &str,
    theirs_label: &str,
) -> ConflictRecord {
    let parent_id = node_parent_in(base, node_id).unwrap_or(NodeId::ROOT);
    ConflictRecord {
        kind: ConflictKind::Lifecycle,
        target: ConflictTarget::Lifecycle { node_id, parent_id },
        base_value: None,
        branches: vec![
            ConflictBranch {
                side: BranchSide::Ours,
                value: serde_json::Value::String(ours_label.into()).into(),
            },
            ConflictBranch {
                side: BranchSide::Theirs,
                value: serde_json::Value::String(theirs_label.into()).into(),
            },
        ],
        auto_resolved: BranchSide::Ours,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::{Node, NodeEntry, NodeId, ParagraphNode, TextNode};

    use super::*;

    #[test]
    fn case_01_both_unchanged_keeps() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.clone();
        let theirs = base.clone();
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&p], NodeDecision::Keep);
    }

    #[test]
    fn case_02_ours_moves_theirs_unchanged() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {
                    text("a")
                }
                p2: paragraph {
                    text("b")
                }
            }
        };
        let txt_id = base.nodes[&p2].children[0];
        let ours = base.with_node_updated(txt_id, |mut e| {
            e.parent = Some(p1);
            e
        });
        let theirs = base.clone();
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(
            result.decisions[&txt_id],
            NodeDecision::MoveTo { parent: p1 }
        );
    }

    #[test]
    fn case_03_theirs_moves_ours_unchanged() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {
                    text("a")
                }
                p2: paragraph {
                    text("b")
                }
            }
        };
        let txt_id = base.nodes[&p2].children[0];
        let ours = base.clone();
        let theirs = base.with_node_updated(txt_id, |mut e| {
            e.parent = Some(p1);
            e
        });
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(
            result.decisions[&txt_id],
            NodeDecision::MoveTo { parent: p1 }
        );
    }

    #[test]
    fn case_04_same_move_no_conflict() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {
                    text("a")
                }
                p2: paragraph {
                    text("b")
                }
            }
        };
        let txt_id = base.nodes[&p2].children[0];
        let moved = base.with_node_updated(txt_id, |mut e| {
            e.parent = Some(p1);
            e
        });
        let (result, conflicts) = merge_tree(&base, &moved, &moved);
        assert!(conflicts.is_empty());
        assert_eq!(
            result.decisions[&txt_id],
            NodeDecision::MoveTo { parent: p1 }
        );
    }

    #[test]
    fn case_05_divergent_move_creates_position_conflict() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {}
                p2: paragraph {}
            }
        };
        let p3 = NodeId::new();
        let base = base.insert_node(
            p3,
            NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT),
        );
        let ours = base.with_node_updated(p3, |mut e| {
            e.parent = Some(p1);
            e
        });
        let theirs = base.with_node_updated(p3, |mut e| {
            e.parent = Some(p2);
            e
        });
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Position);
        assert_eq!(
            conflicts[0].target,
            ConflictTarget::Position { node_id: p3 }
        );
        assert_eq!(result.decisions[&p3], NodeDecision::MoveTo { parent: p1 });
    }

    #[test]
    fn case_06_ours_deletes_theirs_unchanged() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.remove_node(p);
        let theirs = base.clone();
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&p], NodeDecision::Delete);
    }

    #[test]
    fn case_07_theirs_deletes_ours_unchanged() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.clone();
        let theirs = base.remove_node(p);
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&p], NodeDecision::Delete);
    }

    #[test]
    fn case_08_both_delete() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.remove_node(p);
        let theirs = base.remove_node(p);
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&p], NodeDecision::Delete);
    }

    #[test]
    fn case_09_ours_deletes_theirs_edits_lifecycle_conflict() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.remove_node(p);
        let theirs = base.with_node_updated(p, |mut e| {
            e.node = Node::Text(TextNode {
                text: "changed".into(),
            });
            e
        });
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Lifecycle);
        assert_eq!(result.decisions[&p], NodeDecision::Delete);
    }

    #[test]
    fn case_10_ours_edits_theirs_deletes_lifecycle_conflict() {
        let (base, p) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let ours = base.with_node_updated(p, |mut e| {
            e.node = Node::Text(TextNode {
                text: "edited".into(),
            });
            e
        });
        let theirs = base.remove_node(p);
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Lifecycle);
        assert_eq!(result.decisions[&p], NodeDecision::Keep);
    }

    #[test]
    fn case_11_ours_deletes_theirs_moves_lifecycle_conflict() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {}
                p2: paragraph {}
            }
        };
        let p3 = NodeId::new();
        let base = base.insert_node(
            p3,
            NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT),
        );
        let ours = base.remove_node(p3);
        let theirs = base.with_node_updated(p3, |mut e| {
            e.parent = Some(p1);
            e
        });
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Lifecycle);
        assert_eq!(result.decisions[&p3], NodeDecision::Delete);
        let _ = p2;
    }

    #[test]
    fn case_12_ours_moves_theirs_deletes_lifecycle_conflict() {
        let (base, p1, p2) = doc! {
            root {
                p1: paragraph {}
                p2: paragraph {}
            }
        };
        let p3 = NodeId::new();
        let base = base.insert_node(
            p3,
            NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT),
        );
        let ours = base.with_node_updated(p3, |mut e| {
            e.parent = Some(p1);
            e
        });
        let theirs = base.remove_node(p3);
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Lifecycle);
        assert_eq!(result.decisions[&p3], NodeDecision::MoveTo { parent: p1 });
        let _ = p2;
    }

    #[test]
    fn case_13_ours_adds_theirs_absent() {
        let (base,) = doc! { root {} };
        let new_id = NodeId::new();
        let ours = base.insert_node(
            new_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT),
        );
        let theirs = base.clone();
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&new_id], NodeDecision::Add);
    }

    #[test]
    fn case_14_theirs_adds_ours_absent() {
        let (base,) = doc! { root {} };
        let new_id = NodeId::new();
        let ours = base.clone();
        let theirs = base.insert_node(
            new_id,
            NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT),
        );
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&new_id], NodeDecision::Add);
    }

    #[test]
    fn case_15_both_add_same_node_id() {
        let (base,) = doc! { root {} };
        let new_id = NodeId::new();
        let entry = NodeEntry::new(Node::Paragraph(ParagraphNode {})).with_parent(NodeId::ROOT);
        let ours = base.insert_node(new_id, entry.clone());
        let theirs = base.insert_node(new_id, entry);
        let (result, conflicts) = merge_tree(&base, &ours, &theirs);
        assert!(conflicts.is_empty());
        assert_eq!(result.decisions[&new_id], NodeDecision::Add);
    }
}
