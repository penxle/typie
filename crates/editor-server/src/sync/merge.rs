use std::collections::HashSet;

use editor_model::{Doc, NodeEntry, NodeId, TextNode};

use crate::sync::{
    attribute::merge_attribute,
    conflict::{
        AttributeScope, BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
    },
    modifier::merge_modifiers,
    reorder::merge_children_order,
    text::merge_text,
    tree::{NodeDecision, merge_tree},
};

pub fn merge(
    segmenter: &icu_segmenter::GraphemeClusterSegmenter,
    base: &Doc,
    ours: &Doc,
    theirs: &Doc,
) -> (Doc, Vec<ConflictRecord>) {
    let (tree_result, tree_conflicts) = merge_tree(base, ours, theirs);
    let mut all_conflicts: Vec<ConflictRecord> = tree_conflicts;

    let mut merged = base.clone();

    // Sort decisions by NodeId so HashMap iteration order doesn't leak into the merged Doc
    // when concurrent Add decisions compete for the same parent index.
    let mut ordered_decisions: Vec<(NodeId, NodeDecision)> = tree_result
        .decisions
        .iter()
        .map(|(&id, &dec)| (id, dec))
        .collect();
    ordered_decisions.sort_by_key(|(id, _)| *id);

    // Phase 1: process deletions first so removed parents don't block phase 2.
    for &(id, decision) in &ordered_decisions {
        if !matches!(decision, NodeDecision::Delete) {
            continue;
        }
        let parent_id = merged.nodes.get(&id).and_then(|e| e.parent);
        if let Some(p) = parent_id {
            merged = merged.with_node_updated(p, |mut e| {
                e.children.retain(|c| c != &id);
                e
            });
        }
        merged = merged.remove_node(id);
    }

    for &(id, decision) in &ordered_decisions {
        match decision {
            NodeDecision::Add => {
                let (source_entry, source_doc) = ours
                    .nodes
                    .get(&id)
                    .map(|e| (e, ours))
                    .or_else(|| theirs.nodes.get(&id).map(|e| (e, theirs)))
                    .map(|(e, d)| (e.clone(), d))
                    .expect("Add decision requires the node to exist in ours or theirs");
                let parent_id = source_entry
                    .parent
                    .expect("non-root added node must have a parent");
                let insert_index = source_doc
                    .get_entry(parent_id)
                    .and_then(|p| p.children.iter().position(|c| c == &id))
                    .unwrap_or_else(|| {
                        merged
                            .get_entry(parent_id)
                            .map(|p| p.children.len())
                            .unwrap_or(0)
                    });
                // Children that the merge decided to delete must not survive in the new parent's
                // children list, otherwise we leak dangling refs to nodes that won't exist.
                let pruned_children = source_entry
                    .children
                    .iter()
                    .filter(|c| !matches!(tree_result.decisions.get(c), Some(NodeDecision::Delete)))
                    .copied()
                    .collect();
                let source_entry = NodeEntry {
                    children: pruned_children,
                    ..source_entry
                };
                merged = merged.insert_node(id, source_entry);
                // Guard against duplicates: if a parent's source_entry already lists this child
                // (because the parent was also Added and carries its children), skip the insert.
                merged = merged.with_node_updated(parent_id, |mut e| {
                    if !e.children.iter().any(|c| c == &id) {
                        let idx = insert_index.min(e.children.len());
                        e.children.insert(idx, id);
                    }
                    e
                });
            }
            NodeDecision::MoveTo { parent: new_parent } => {
                if matches!(
                    tree_result.decisions.get(&new_parent),
                    Some(NodeDecision::Delete)
                ) {
                    // The intended parent was concurrently deleted on the opposing side. Keep the
                    // node at its base location and surface the structural disagreement. If the
                    // base parent was also deleted, reparent to ROOT so the node has a live parent.
                    let base_parent = base
                        .nodes
                        .get(&id)
                        .and_then(|e| e.parent)
                        .unwrap_or(NodeId::ROOT);
                    let safe_parent = if matches!(
                        tree_result.decisions.get(&base_parent),
                        Some(NodeDecision::Delete)
                    ) {
                        NodeId::ROOT
                    } else {
                        base_parent
                    };
                    all_conflicts.push(ConflictRecord {
                        kind: ConflictKind::Position,
                        target: ConflictTarget::Position { node_id: id },
                        base_value: serde_json::to_value(base_parent).ok().map(Into::into),
                        branches: vec![
                            ConflictBranch {
                                side: BranchSide::Ours,
                                value: serde_json::to_value(new_parent).unwrap().into(),
                            },
                            ConflictBranch {
                                side: BranchSide::Theirs,
                                value: serde_json::to_value(safe_parent).unwrap().into(),
                            },
                        ],
                        auto_resolved: BranchSide::Ours,
                    });
                    if safe_parent != base_parent {
                        merged = merged.with_node_updated(id, |mut e| {
                            e.parent = Some(safe_parent);
                            e
                        });
                        merged = merged.with_node_updated(safe_parent, |mut e| {
                            if !e.children.iter().any(|c| c == &id) {
                                e.children.push_back(id);
                            }
                            e
                        });
                    }
                    continue;
                }
                let old_parent = merged.nodes.get(&id).and_then(|e| e.parent);
                if let Some(op) = old_parent {
                    merged = merged.with_node_updated(op, |mut e| {
                        e.children.retain(|c| c != &id);
                        e
                    });
                }
                merged = merged.with_node_updated(id, |mut e| {
                    e.parent = Some(new_parent);
                    e
                });
                let insert_index = ours
                    .get_entry(new_parent)
                    .and_then(|p| p.children.iter().position(|c| c == &id))
                    .or_else(|| {
                        theirs
                            .get_entry(new_parent)
                            .and_then(|p| p.children.iter().position(|c| c == &id))
                    })
                    .unwrap_or_else(|| {
                        merged
                            .get_entry(new_parent)
                            .map(|p| p.children.len())
                            .unwrap_or(0)
                    });
                merged = merged.with_node_updated(new_parent, |mut e| {
                    if !e.children.iter().any(|c| c == &id) {
                        let idx = insert_index.min(e.children.len());
                        e.children.insert(idx, id);
                    }
                    e
                });
            }
            NodeDecision::Delete | NodeDecision::Keep => {}
        }
    }

    for &(node_id, decision) in &ordered_decisions {
        if !matches!(decision, NodeDecision::Keep | NodeDecision::MoveTo { .. }) {
            continue;
        }

        let b_entry = base.nodes.get(&node_id);
        let o_entry = ours.nodes.get(&node_id);
        let t_entry = theirs.nodes.get(&node_id);

        // The lifecycle conflict cases (Move/Edit on one side vs Delete on the other) reach this
        // loop with one of o/t being None. Take the surviving side's content rather than letting
        // base content silently overwrite the edits that won the conflict.
        let (b_entry, o_entry, t_entry) = match (b_entry, o_entry, t_entry) {
            (Some(b), Some(o), Some(t)) => (b, o, t),
            (Some(_), Some(o), None) => {
                merged = merged.with_node_updated(node_id, |mut e| {
                    e.node = o.node.clone();
                    e.modifiers = o.modifiers.clone();
                    e
                });
                continue;
            }
            (Some(_), None, Some(t)) => {
                merged = merged.with_node_updated(node_id, |mut e| {
                    e.node = t.node.clone();
                    e.modifiers = t.modifiers.clone();
                    e
                });
                continue;
            }
            _ => continue,
        };

        let merged_node = match (&b_entry.node, &o_entry.node, &t_entry.node) {
            (
                editor_model::Node::Text(bt),
                editor_model::Node::Text(ot),
                editor_model::Node::Text(tt),
            ) => {
                let (text, text_conflicts) =
                    merge_text(segmenter, node_id, &bt.text, &ot.text, &tt.text);
                all_conflicts.extend(text_conflicts);
                editor_model::Node::Text(TextNode { text })
            }
            _ => {
                let (val, conflict) = merge_attribute(
                    ConflictTarget::Attribute {
                        scope: AttributeScope::Node { node_id },
                        name: "node_value".into(),
                    },
                    Some(&serde_json::to_value(&b_entry.node).unwrap()),
                    Some(&serde_json::to_value(&o_entry.node).unwrap()),
                    Some(&serde_json::to_value(&t_entry.node).unwrap()),
                );
                all_conflicts.extend(conflict);
                serde_json::from_value(val.unwrap()).unwrap()
            }
        };

        let (merged_modifiers, mod_conflicts) = merge_modifiers(
            node_id,
            &b_entry.modifiers,
            &o_entry.modifiers,
            &t_entry.modifiers,
        );
        all_conflicts.extend(mod_conflicts);

        let base_children_set: HashSet<NodeId> = b_entry.children.iter().copied().collect();
        let ours_children_set: HashSet<NodeId> = o_entry.children.iter().copied().collect();
        let theirs_children_set: HashSet<NodeId> = t_entry.children.iter().copied().collect();

        let merged_children =
            if base_children_set == ours_children_set && base_children_set == theirs_children_set {
                let bv: Vec<NodeId> = b_entry.children.iter().copied().collect();
                let ov: Vec<NodeId> = o_entry.children.iter().copied().collect();
                let tv: Vec<NodeId> = t_entry.children.iter().copied().collect();
                let (order, conflict) = merge_children_order(node_id, &bv, &ov, &tv);
                all_conflicts.extend(conflict);
                Some(editor_model::imbl::Vector::from_iter(order))
            } else {
                None
            };

        merged = merged.with_node_updated(node_id, |mut e| {
            e.node = merged_node;
            e.modifiers = merged_modifiers;
            if let Some(ch) = merged_children {
                e.children = ch;
            }
            e
        });
    }

    (merged, all_conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;
    use editor_model::{
        BulletListNode, CalloutNode, CalloutVariant, LayoutMode, ListItemNode, Modifier, Node,
        ParagraphNode, RootNode, TableCellNode, TableNode, TableRowNode,
    };
    use icu_segmenter::GraphemeClusterSegmenter;

    fn segmenter() -> GraphemeClusterSegmenter {
        GraphemeClusterSegmenter::new().static_to_owned()
    }

    #[test]
    fn modifier_change_and_text_change_auto_merge() {
        let (base, p, t) = doc! {
            root {
                p: paragraph {
                    t: text("hello")
                }
            }
        };
        let ours = base.with_node_updated(p, |mut e| {
            e.modifiers.push(Modifier::Bold);
            e
        });
        let theirs = base.with_node_updated(t, |mut e| {
            if let editor_model::Node::Text(ref mut tn) = e.node {
                tn.text = "hello world".to_string();
            }
            e
        });
        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );

        let p_entry = merged.get_entry(p).unwrap();
        assert!(p_entry.modifiers.contains(&Modifier::Bold));

        let t_entry = merged.get_entry(t).unwrap();
        match &t_entry.node {
            editor_model::Node::Text(tn) => assert_eq!(tn.text, "hello world"),
            other => panic!("expected text node, got {:?}", other),
        }
    }

    #[test]
    fn add_decision_keeps_parent_children_consistent() {
        let (base, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                }
            }
        };
        let new_p = NodeId::new();
        let new_p_entry =
            NodeEntry::new(Node::Paragraph(ParagraphNode::default())).with_parent(NodeId::ROOT);
        let ours = base
            .insert_node(new_p, new_p_entry)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(new_p);
                e
            });
        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        assert!(merged.nodes.contains_key(&new_p));
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn delete_decision_keeps_parent_children_consistent() {
        let (base, p, ..) = doc! {
            root {
                p: paragraph {
                    text("hello")
                }
            }
        };
        let t_id = base.get_entry(p).unwrap().children[0];
        let ours =
            base.remove_node(p)
                .remove_node(t_id)
                .with_node_updated(NodeId::ROOT, |mut e| {
                    e.children.retain(|c| c != &p);
                    e
                });
        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        assert!(!merged.nodes.contains_key(&p));
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn move_decision_keeps_parent_children_consistent() {
        let (base, p1, t1, p2) = doc! {
            root {
                p1: paragraph {
                    t1: text("hello")
                }
                p2: paragraph {}
            }
        };
        let ours = base
            .with_node_updated(t1, |mut e| {
                e.parent = Some(p2);
                e
            })
            .with_node_updated(p1, |mut e| {
                e.children.retain(|c| c != &t1);
                e
            })
            .with_node_updated(p2, |mut e| {
                e.children.push_back(t1);
                e
            });
        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        let t1_entry = merged.get_entry(t1).unwrap();
        assert_eq!(t1_entry.parent, Some(p2));
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn root_layout_one_side_changes_takes_change() {
        let (base, ..) = doc! {
            root {
                paragraph {
                    text("hi")
                }
            }
        };
        let new_root = Node::Root(RootNode {
            layout_mode: LayoutMode::Paginated {
                page_width: 595.0,
                page_height: 842.0,
                page_margin_top: 50.0,
                page_margin_bottom: 50.0,
                page_margin_left: 50.0,
                page_margin_right: 50.0,
            },
        });
        let ours = base.with_node_updated(NodeId::ROOT, |mut e| {
            e.node = new_root.clone();
            e
        });
        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        let merged_root = &merged.get_entry(NodeId::ROOT).unwrap().node;
        assert_eq!(merged_root, &new_root);
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn root_layout_both_same_change_no_conflict() {
        let (base, ..) = doc! {
            root {
                paragraph {
                    text("hi")
                }
            }
        };
        let new_root = Node::Root(RootNode {
            layout_mode: LayoutMode::Continuous { max_width: 800.0 },
        });
        let ours = base.with_node_updated(NodeId::ROOT, |mut e| {
            e.node = new_root.clone();
            e
        });
        let theirs = base.with_node_updated(NodeId::ROOT, |mut e| {
            e.node = new_root.clone();
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        let merged_root = &merged.get_entry(NodeId::ROOT).unwrap().node;
        assert_eq!(merged_root, &new_root);
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn root_layout_both_change_to_different_values_creates_conflict() {
        let (base, ..) = doc! {
            root {
                paragraph {
                    text("hi")
                }
            }
        };
        let ours_root = Node::Root(RootNode {
            layout_mode: LayoutMode::Paginated {
                page_width: 595.0,
                page_height: 842.0,
                page_margin_top: 50.0,
                page_margin_bottom: 50.0,
                page_margin_left: 50.0,
                page_margin_right: 50.0,
            },
        });
        let theirs_root = Node::Root(RootNode {
            layout_mode: LayoutMode::Continuous { max_width: 800.0 },
        });
        let ours = base.with_node_updated(NodeId::ROOT, |mut e| {
            e.node = ours_root.clone();
            e
        });
        let theirs = base.with_node_updated(NodeId::ROOT, |mut e| {
            e.node = theirs_root.clone();
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert_eq!(
            conflicts.len(),
            1,
            "expected 1 conflict, got {:?}",
            conflicts
        );
        assert_eq!(
            conflicts[0].kind,
            crate::sync::conflict::ConflictKind::Attribute
        );
        match &conflicts[0].target {
            crate::sync::conflict::ConflictTarget::Attribute {
                scope: crate::sync::conflict::AttributeScope::Node { node_id },
                ..
            } => assert_eq!(*node_id, NodeId::ROOT),
            other => panic!(
                "expected node-scope attribute conflict on ROOT, got {:?}",
                other
            ),
        }
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn move_subtree_preserves_children() {
        let (base, c1, p1, t1, c2) = doc! {
            root {
                c1: callout {
                    p1: paragraph {
                        t1: text("hello")
                    }
                }
                c2: callout {}
            }
        };

        let ours = base
            .with_node_updated(p1, |mut e| {
                e.parent = Some(c2);
                e
            })
            .with_node_updated(c1, |mut e| {
                e.children.retain(|c| c != &p1);
                e
            })
            .with_node_updated(c2, |mut e| {
                e.children.push_back(p1);
                e
            });
        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );

        let p1_entry = merged.get_entry(p1).unwrap();
        assert_eq!(p1_entry.parent, Some(c2));
        assert_eq!(p1_entry.children.len(), 1);
        assert_eq!(p1_entry.children[0], t1);

        let t1_entry = merged.get_entry(t1).unwrap();
        assert_eq!(t1_entry.parent, Some(p1));

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn concurrent_add_delete_edit_combine() {
        let (base, p1, t1, p2, t2) = doc! {
            root {
                p1: paragraph {
                    t1: text("hello")
                }
                p2: paragraph {
                    t2: text("world")
                }
            }
        };

        let ours = base
            .remove_node(t1)
            .remove_node(p1)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.retain(|c| c != &p1);
                e
            })
            .with_node_updated(p2, |mut e| {
                e.modifiers.push(Modifier::Bold);
                e
            });

        let p3 = NodeId::new();
        let t3 = NodeId::new();
        let theirs = base
            .insert_node(
                t3,
                NodeEntry::new(Node::Text(TextNode { text: "new".into() })).with_parent(p3),
            )
            .insert_node(
                p3,
                NodeEntry::new(Node::Paragraph(ParagraphNode::default()))
                    .with_parent(NodeId::ROOT)
                    .with_children(editor_model::imbl::Vector::from(vec![t3])),
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(p3);
                e
            })
            .with_node_updated(t2, |mut e| {
                if let Node::Text(ref mut tn) = e.node {
                    tn.text = "world!".into();
                }
                e
            });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );

        assert!(!merged.nodes.contains_key(&p1));
        assert!(!merged.nodes.contains_key(&t1));
        assert!(merged.nodes.contains_key(&p3));
        assert!(merged.nodes.contains_key(&t3));

        let p2_entry = merged.get_entry(p2).unwrap();
        assert!(p2_entry.modifiers.contains(&Modifier::Bold));

        let t2_entry = merged.get_entry(t2).unwrap();
        if let Node::Text(tn) = &t2_entry.node {
            assert_eq!(tn.text, "world!");
        } else {
            panic!("expected text node");
        }

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn concurrent_adds_at_same_parent_both_appear() {
        let (base, ..) = doc! {
            root {
                paragraph {
                    text("hi")
                }
            }
        };

        let new_p_ours = NodeId::new();
        let ours = base
            .insert_node(
                new_p_ours,
                NodeEntry::new(Node::Paragraph(ParagraphNode::default())).with_parent(NodeId::ROOT),
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(new_p_ours);
                e
            });

        let new_p_theirs = NodeId::new();
        let theirs = base
            .insert_node(
                new_p_theirs,
                NodeEntry::new(Node::Paragraph(ParagraphNode::default())).with_parent(NodeId::ROOT),
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(new_p_theirs);
                e
            });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );
        assert!(merged.nodes.contains_key(&new_p_ours));
        assert!(merged.nodes.contains_key(&new_p_theirs));
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn ours_moves_and_edits_theirs_deletes_keeps_ours_edit() {
        let (base, p, t) = doc! {
            root {
                p: paragraph {
                    t: text("hello")
                }
            }
        };
        let p_new = NodeId::new();
        let p_new_entry =
            NodeEntry::new(Node::Paragraph(ParagraphNode::default())).with_parent(NodeId::ROOT);
        let ours = base
            .insert_node(p_new, p_new_entry)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(p_new);
                e
            })
            .with_node_updated(t, |mut e| {
                e.parent = Some(p_new);
                if let Node::Text(ref mut tn) = e.node {
                    tn.text = "edited".into();
                }
                e
            })
            .with_node_updated(p, |mut e| {
                e.children.retain(|c| c != &t);
                e
            })
            .with_node_updated(p_new, |mut e| {
                e.children.push_back(t);
                e
            });

        let theirs = base.remove_node(t).with_node_updated(p, |mut e| {
            e.children.retain(|c| c != &t);
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0].kind,
            crate::sync::conflict::ConflictKind::Lifecycle
        );

        let t_entry = merged.get_entry(t).expect("t survives the conflict");
        assert_eq!(t_entry.parent, Some(p_new));
        match &t_entry.node {
            Node::Text(tn) => assert_eq!(tn.text, "edited"),
            other => panic!("expected text node, got {:?}", other),
        }
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn ours_moves_to_target_theirs_deletes_target_keeps_node_at_base() {
        let (base, c1, p, t, c2) = doc! {
            root {
                c1: callout {
                    p: paragraph {
                        t: text("hello")
                    }
                }
                c2: callout {}
            }
        };

        let ours = base
            .with_node_updated(p, |mut e| {
                e.parent = Some(c2);
                e
            })
            .with_node_updated(c1, |mut e| {
                e.children.retain(|c| c != &p);
                e
            })
            .with_node_updated(c2, |mut e| {
                e.children.push_back(p);
                e
            });

        let theirs = base
            .remove_node(c2)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.retain(|c| c != &c2);
                e
            });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);

        assert!(
            conflicts
                .iter()
                .any(|c| matches!(c.kind, crate::sync::conflict::ConflictKind::Position)),
            "expected a position conflict, got {:?}",
            conflicts
        );

        let p_entry = merged.get_entry(p).unwrap();
        assert_eq!(p_entry.parent, Some(c1), "p must remain at base location");
        assert!(!merged.nodes.contains_key(&c2), "c2 was deleted");
        let t_entry = merged.get_entry(t).unwrap();
        assert_eq!(t_entry.parent, Some(p), "t still under p");

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn ours_moves_theirs_deletes_both_target_and_base_parent_reparents_to_root() {
        let (base, p1, p2, t) = doc! {
            root {
                p1: paragraph {
                    t: text("hello")
                }
                p2: paragraph {}
            }
        };

        let ours = base
            .with_node_updated(t, |mut e| {
                e.parent = Some(p2);
                e
            })
            .with_node_updated(p1, |mut e| {
                e.children.retain(|c| c != &t);
                e
            })
            .with_node_updated(p2, |mut e| {
                e.children.push_back(t);
                e
            });

        let theirs = base
            .with_node_updated(p1, |mut e| {
                e.children.retain(|c| c != &t);
                e
            })
            .remove_node(t)
            .remove_node(p1)
            .remove_node(p2)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.retain(|c| c != &p1 && c != &p2);
                e
            });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);

        let t_entry = merged
            .get_entry(t)
            .expect("t survives via lifecycle conflict");
        assert_eq!(
            t_entry.parent,
            Some(NodeId::ROOT),
            "t reparented to ROOT because both target P2 and base parent P1 were deleted"
        );
        assert!(!merged.nodes.contains_key(&p1));
        assert!(!merged.nodes.contains_key(&p2));

        assert!(
            conflicts
                .iter()
                .any(|c| matches!(c.kind, crate::sync::conflict::ConflictKind::Position)),
            "expected position conflict for the move-to-deleted-target case, got {:?}",
            conflicts
        );

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn ours_deletes_theirs_adds_parent_owning_deleted_no_dangling() {
        let (base, _p, t) = doc! {
            root {
                _p: paragraph {
                    t: text("hello")
                }
            }
        };
        let ours = base.remove_node(t).with_node_updated(
            base.get_entry(t).unwrap().parent.unwrap(),
            |mut e| {
                e.children.retain(|c| c != &t);
                e
            },
        );

        let p_new = NodeId::new();
        let p_new_entry = NodeEntry::new(Node::Paragraph(ParagraphNode::default()))
            .with_parent(NodeId::ROOT)
            .with_children(editor_model::imbl::Vector::from(vec![t]));
        let theirs = base
            .insert_node(p_new, p_new_entry)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(p_new);
                e
            })
            .with_node_updated(t, |mut e| {
                e.parent = Some(p_new);
                e
            })
            .with_node_updated(base.get_entry(t).unwrap().parent.unwrap(), |mut e| {
                e.children.retain(|c| c != &t);
                e
            });

        let (merged, _conflicts) = merge(&segmenter(), &base, &ours, &theirs);

        assert!(!merged.nodes.contains_key(&t), "t was deleted by ours");
        let p_new_entry = merged.get_entry(p_new).expect("p_new was added by theirs");
        assert!(
            !p_new_entry.children.iter().any(|c| c == &t),
            "p_new.children must not reference deleted t"
        );
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn add_parent_with_moved_existing_child_no_duplicate() {
        let (base, p_old, t) = doc! {
            root {
                p_old: paragraph {
                    t: text("hello")
                }
            }
        };
        let p_new = NodeId::new();
        let p_new_entry = NodeEntry::new(Node::Paragraph(ParagraphNode::default()))
            .with_parent(NodeId::ROOT)
            .with_children(editor_model::imbl::Vector::from(vec![t]));
        let ours = base
            .insert_node(p_new, p_new_entry)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(p_new);
                e
            })
            .with_node_updated(p_old, |mut e| {
                e.children.retain(|c| c != &t);
                e
            })
            .with_node_updated(t, |mut e| {
                e.parent = Some(p_new);
                e
            });

        let theirs = base.clone();

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(
            conflicts.is_empty(),
            "expected no conflicts, got {:?}",
            conflicts
        );

        let p_new_entry = merged.get_entry(p_new).unwrap();
        let occurrences = p_new_entry.children.iter().filter(|c| **c == t).count();
        assert_eq!(
            occurrences, 1,
            "t must appear exactly once in p_new.children"
        );
        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn merge_inside_callout_container() {
        let p_id = NodeId::new();
        let t_id = NodeId::new();
        let c_id = NodeId::new();

        let base = editor_model::Doc::default()
            .insert_node(
                c_id,
                NodeEntry {
                    node: Node::Callout(CalloutNode {
                        variant: CalloutVariant::Info,
                    }),
                    parent: Some(NodeId::ROOT),
                    children: editor_model::imbl::Vector::from(vec![p_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                p_id,
                NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(c_id),
                    children: editor_model::imbl::Vector::from(vec![t_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                t_id,
                NodeEntry {
                    node: Node::Text(TextNode {
                        text: "hello".into(),
                    }),
                    parent: Some(p_id),
                    children: editor_model::imbl::Vector::new(),
                    modifiers: vec![],
                },
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(c_id);
                e
            });

        let ours = base.with_node_updated(c_id, |mut e| {
            e.node = Node::Callout(CalloutNode {
                variant: CalloutVariant::Warning,
            });
            e
        });
        let theirs = base.with_node_updated(t_id, |mut e| {
            if let Node::Text(ref mut tn) = e.node {
                tn.text = "warned".into();
            }
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(conflicts.is_empty(), "got conflicts: {:?}", conflicts);

        let c_entry = merged.get_entry(c_id).unwrap();
        if let Node::Callout(cn) = &c_entry.node {
            assert_eq!(cn.variant, CalloutVariant::Warning);
        } else {
            panic!("expected callout");
        }

        let t_entry = merged.get_entry(t_id).unwrap();
        if let Node::Text(tn) = &t_entry.node {
            assert_eq!(tn.text, "warned");
        } else {
            panic!("expected text");
        }

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn merge_inside_table_structure() {
        let table_id = NodeId::new();
        let row_id = NodeId::new();
        let cell_id = NodeId::new();
        let p_id = NodeId::new();
        let t_id = NodeId::new();

        let base = editor_model::Doc::default()
            .insert_node(
                table_id,
                NodeEntry {
                    node: Node::Table(TableNode::default()),
                    parent: Some(NodeId::ROOT),
                    children: editor_model::imbl::Vector::from(vec![row_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                row_id,
                NodeEntry {
                    node: Node::TableRow(TableRowNode {}),
                    parent: Some(table_id),
                    children: editor_model::imbl::Vector::from(vec![cell_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                cell_id,
                NodeEntry {
                    node: Node::TableCell(TableCellNode {
                        col_width: Some(100.0),
                    }),
                    parent: Some(row_id),
                    children: editor_model::imbl::Vector::from(vec![p_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                p_id,
                NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(cell_id),
                    children: editor_model::imbl::Vector::from(vec![t_id]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                t_id,
                NodeEntry {
                    node: Node::Text(TextNode {
                        text: "cell".into(),
                    }),
                    parent: Some(p_id),
                    children: editor_model::imbl::Vector::new(),
                    modifiers: vec![],
                },
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(table_id);
                e
            });

        let ours = base.with_node_updated(t_id, |mut e| {
            if let Node::Text(ref mut tn) = e.node {
                tn.text = "edited".into();
            }
            e
        });
        let theirs = base.with_node_updated(cell_id, |mut e| {
            e.node = Node::TableCell(TableCellNode {
                col_width: Some(200.0),
            });
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(conflicts.is_empty(), "got conflicts: {:?}", conflicts);

        let cell_entry = merged.get_entry(cell_id).unwrap();
        if let Node::TableCell(cn) = &cell_entry.node {
            assert_eq!(cn.col_width, Some(200.0));
        } else {
            panic!("expected table cell");
        }

        let t_entry = merged.get_entry(t_id).unwrap();
        if let Node::Text(tn) = &t_entry.node {
            assert_eq!(tn.text, "edited");
        } else {
            panic!("expected text");
        }

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }

    #[test]
    fn merge_inside_list_structure() {
        let list_id = NodeId::new();
        let li1 = NodeId::new();
        let li2 = NodeId::new();
        let p1 = NodeId::new();
        let p2 = NodeId::new();
        let t1 = NodeId::new();
        let t2 = NodeId::new();

        let base = editor_model::Doc::default()
            .insert_node(
                list_id,
                NodeEntry {
                    node: Node::BulletList(BulletListNode {}),
                    parent: Some(NodeId::ROOT),
                    children: editor_model::imbl::Vector::from(vec![li1, li2]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                li1,
                NodeEntry {
                    node: Node::ListItem(ListItemNode {}),
                    parent: Some(list_id),
                    children: editor_model::imbl::Vector::from(vec![p1]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                li2,
                NodeEntry {
                    node: Node::ListItem(ListItemNode {}),
                    parent: Some(list_id),
                    children: editor_model::imbl::Vector::from(vec![p2]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                p1,
                NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(li1),
                    children: editor_model::imbl::Vector::from(vec![t1]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                p2,
                NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(li2),
                    children: editor_model::imbl::Vector::from(vec![t2]),
                    modifiers: vec![],
                },
            )
            .insert_node(
                t1,
                NodeEntry {
                    node: Node::Text(TextNode {
                        text: "first".into(),
                    }),
                    parent: Some(p1),
                    children: editor_model::imbl::Vector::new(),
                    modifiers: vec![],
                },
            )
            .insert_node(
                t2,
                NodeEntry {
                    node: Node::Text(TextNode {
                        text: "second".into(),
                    }),
                    parent: Some(p2),
                    children: editor_model::imbl::Vector::new(),
                    modifiers: vec![],
                },
            )
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.push_back(list_id);
                e
            });

        let ours = base.with_node_updated(list_id, |mut e| {
            e.children = editor_model::imbl::Vector::from(vec![li2, li1]);
            e
        });
        let theirs = base.with_node_updated(t1, |mut e| {
            if let Node::Text(ref mut tn) = e.node {
                tn.text = "edited".into();
            }
            e
        });

        let (merged, conflicts) = merge(&segmenter(), &base, &ours, &theirs);
        assert!(conflicts.is_empty(), "got conflicts: {:?}", conflicts);

        let list_entry = merged.get_entry(list_id).unwrap();
        assert_eq!(
            list_entry.children.iter().copied().collect::<Vec<_>>(),
            vec![li2, li1]
        );

        let t1_entry = merged.get_entry(t1).unwrap();
        if let Node::Text(tn) = &t1_entry.node {
            assert_eq!(tn.text, "edited");
        } else {
            panic!("expected text");
        }

        crate::sync::test_helpers::assert_doc_consistent(&merged);
    }
}
