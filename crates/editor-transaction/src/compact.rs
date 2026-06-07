use editor_model::*;

use crate::Step;

/// Analyzes a node's children and returns steps to normalize adjacent text nodes.
/// Removes empty text nodes via RemoveSubtree, then merges consecutive text nodes
/// with the same modifier set via MergeNode. Returns empty vec if already normalized.
pub fn compact(node: &NodeRef) -> Vec<Step> {
    let children: Vec<_> = node.children().collect();
    let mut steps = Vec::new();

    // Pass 1: remove empty text nodes (reverse order for index stability)
    for i in (0..children.len()).rev() {
        if let Node::Text(t) = children[i].node()
            && t.text.is_empty()
        {
            steps.push(Step::RemoveSubtree {
                parent_id: node.id(),
                index: i,
                subtree: Subtree {
                    id: children[i].id(),
                    node: children[i].node().to_plain(),
                    modifiers: children[i].modifiers().cloned().collect(),
                    style: children[i].entry().style.get().clone(),
                    marker: children[i].entry().marker.get().clone(),
                    children: vec![],
                },
            });
        }
    }

    // Pass 2: merge consecutive same-modifier text nodes among non-empty remainder (reverse)
    let remaining: Vec<_> = children
        .iter()
        .filter(|c| !matches!(c.node(), Node::Text(t) if t.text.is_empty()))
        .collect();

    for i in (1..remaining.len()).rev() {
        let curr = remaining[i];
        let prev = remaining[i - 1];

        let (Node::Text(_), Node::Text(prev_text)) = (curr.node(), prev.node()) else {
            continue;
        };

        let curr_mods: Vec<Modifier> = curr.modifiers().cloned().collect();
        let prev_mods: Vec<Modifier> = prev.modifiers().cloned().collect();
        if !modifiers_set_eq(&curr_mods, &prev_mods) {
            continue;
        }
        if curr.entry().style.get() != prev.entry().style.get() {
            continue;
        }

        steps.push(Step::MergeNode {
            node_id: curr.id(),
            target_id: prev.id(),
            offset: prev_text.text.len(),
        });
    }

    steps
}

fn modifiers_set_eq(a: &[Modifier], b: &[Modifier]) -> bool {
    a.len() == b.len() && a.iter().all(|m| b.contains(m))
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};

    use crate::Transaction;
    use crate::test_utils::DocTestExt;

    use super::*;

    #[test]
    fn remove_empty_text_node() {
        let (doc, p1, t2, ..) = doc! {
            root {
                p1: paragraph {
                    text("A")
                    t2: text("")
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, p1);
                assert_eq!(*index, 1);
                assert_eq!(subtree.id, t2);
            }
            _ => panic!("expected RemoveSubtree"),
        }
    }

    #[test]
    fn remove_multiple_empty_text_nodes() {
        let (doc, p1, t1, t3, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("")
                    text("A")
                    t3: text("")
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 2);
        // Reverse order: t3 first, then t1
        match &steps[0] {
            Step::RemoveSubtree { index, subtree, .. } => {
                assert_eq!(*index, 2);
                assert_eq!(subtree.id, t3);
            }
            _ => panic!("expected RemoveSubtree for t3"),
        }
        match &steps[1] {
            Step::RemoveSubtree { index, subtree, .. } => {
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, t1);
            }
            _ => panic!("expected RemoveSubtree for t1"),
        }
    }

    #[test]
    fn no_empty_text_returns_empty() {
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph {
                    text("A") [bold]
                    text("B") [italic]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);
        assert!(steps.is_empty());
    }

    #[test]
    fn empty_paragraph_returns_empty() {
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);
        assert!(steps.is_empty());
    }

    #[test]
    fn merge_same_modifier_text_nodes() {
        let (doc, p1, t1, t2, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("Hello") [bold]
                    t2: text(" World") [bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => {
                assert_eq!(*node_id, t2);
                assert_eq!(*target_id, t1);
                assert_eq!(*offset, 5);
            }
            _ => panic!("expected MergeNode"),
        }
    }

    #[test]
    fn no_merge_different_modifiers() {
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph {
                    text("A") [bold]
                    text("B") [italic]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);
        assert!(steps.is_empty());
    }

    #[test]
    fn merge_three_consecutive_same_modifier() {
        let (doc, p1, t1, t2, t3, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("A") [bold]
                    t2: text("B") [bold]
                    t3: text("C") [bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 2);
        // Reverse order: t3→t2 first, then t2→t1
        match &steps[0] {
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => {
                assert_eq!(*node_id, t3);
                assert_eq!(*target_id, t2);
                assert_eq!(*offset, 1);
            }
            _ => panic!("expected MergeNode t3→t2"),
        }
        match &steps[1] {
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => {
                assert_eq!(*node_id, t2);
                assert_eq!(*target_id, t1);
                assert_eq!(*offset, 1);
            }
            _ => panic!("expected MergeNode t2→t1"),
        }
    }

    #[test]
    fn merge_unmodified_text_nodes() {
        let (doc, p1, t1, t2, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("A")
                    t2: text("B")
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::MergeNode {
                node_id, target_id, ..
            } => {
                assert_eq!(*node_id, t2);
                assert_eq!(*target_id, t1);
            }
            _ => panic!("expected MergeNode"),
        }
    }

    #[test]
    fn no_merge_non_text_between() {
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph {
                    text("A") [bold]
                    hard_break
                    text("B") [bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);
        assert!(steps.is_empty());
    }

    #[test]
    fn merge_modifier_order_independent() {
        let (doc, p1, t1, t2, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("A") [bold, italic]
                    t2: text("B") [italic, bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::MergeNode {
                node_id, target_id, ..
            } => {
                assert_eq!(*node_id, t2);
                assert_eq!(*target_id, t1);
            }
            _ => panic!("expected MergeNode"),
        }
    }

    #[test]
    fn remove_empty_then_merge_adjacent() {
        let (doc, p1, t1, t2, t3, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("A") [bold]
                    t2: text("") [italic]
                    t3: text("B") [bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 2);
        // Pass 1: RemoveSubtree(t2)
        match &steps[0] {
            Step::RemoveSubtree { subtree, .. } => {
                assert_eq!(subtree.id, t2);
            }
            _ => panic!("expected RemoveSubtree for t2"),
        }
        // Pass 2: MergeNode(t3→t1)
        match &steps[1] {
            Step::MergeNode {
                node_id, target_id, ..
            } => {
                assert_eq!(*node_id, t3);
                assert_eq!(*target_id, t1);
            }
            _ => panic!("expected MergeNode t3→t1"),
        }
    }

    #[test]
    fn remove_empty_and_merge_three() {
        let (doc, p1, t1, t2, t3, t4, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("A") [bold]
                    t2: text("")
                    t3: text("B") [bold]
                    t4: text("C") [bold]
                }
            }
        };

        let p = doc.node(p1).unwrap();
        let steps = compact(&p);

        assert_eq!(steps.len(), 3);
        // Pass 1: RemoveSubtree(t2)
        match &steps[0] {
            Step::RemoveSubtree { subtree, .. } => {
                assert_eq!(subtree.id, t2);
            }
            _ => panic!("expected RemoveSubtree"),
        }
        // Pass 2: MergeNode(t4→t3), MergeNode(t3→t1)
        match &steps[1] {
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => {
                assert_eq!(*node_id, t4);
                assert_eq!(*target_id, t3);
                assert_eq!(*offset, 1);
            }
            _ => panic!("expected MergeNode t4→t3"),
        }
        match &steps[2] {
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => {
                assert_eq!(*node_id, t3);
                assert_eq!(*target_id, t1);
                assert_eq!(*offset, 1);
            }
            _ => panic!("expected MergeNode t3→t1"),
        }
    }

    #[test]
    fn compact_steps_apply_and_undo() {
        let (state, p1, t1, t2, t3, t4) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("A") [bold]
                        t2: text("") [italic]
                        t3: text("B") [bold]
                        t4: text("C") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };

        let p = state.doc.node(p1).unwrap();
        let steps = compact(&p);
        assert_eq!(steps.len(), 3);

        // Apply all steps
        let mut tr = Transaction::new(&state);
        tr.apply_steps(steps.clone()).unwrap();
        let (new_state, _, _, _, _) = tr.commit();

        // After compact: t1 = "ABC", t2/t3/t4 removed
        assert_eq!(new_state.text(t1).text.to_string(), "ABC");
        assert!(!new_state.has_node(t2));
        assert!(!new_state.has_node(t3));
        assert!(!new_state.has_node(t4));
        assert_eq!(new_state.node(p1).children().count(), 1);

        // Undo: apply inverse steps in reverse order
        let inverse_steps: Vec<_> = steps.iter().rev().map(|s| s.inverse()).collect();
        let mut tr2 = Transaction::new(&new_state);
        tr2.apply_steps(inverse_steps).unwrap();
        let (restored, _, _, _, _) = tr2.commit();

        // Restored state should match original
        assert_eq!(restored.text(t1).text.to_string(), "A");
        assert!(restored.has_node(t2));
        assert_eq!(restored.text(t3).text.to_string(), "B");
        assert_eq!(restored.text(t4).text.to_string(), "C");
        assert_eq!(restored.node(p1).children().count(), 4);
    }

    #[test]
    fn no_merge_when_style_refs_differ() {
        use editor_model::PlainStyleEntry;
        let (initial, _p1, t1, _t2) = state! {
            doc { root { p1: paragraph { t1: text("A") t2: text("B") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "s1".into(),
            Some(PlainStyleEntry {
                name: "s".into(),
                modifiers: Default::default(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("s1".into())).unwrap();
        let (next, ..) = tr.commit();

        let parent_id = next.doc.node(t1).unwrap().parent().unwrap().id();
        let p = next.doc.node(parent_id).unwrap();
        let steps = compact(&p);
        assert!(
            steps.is_empty(),
            "runs with different style refs must not merge"
        );
    }

    #[test]
    fn merge_when_style_refs_equal() {
        use editor_model::PlainStyleEntry;
        let (initial, _p1, t1, t2) = state! {
            doc { root { p1: paragraph { t1: text("A") t2: text("B") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "s1".into(),
            Some(PlainStyleEntry {
                name: "s".into(),
                modifiers: Default::default(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("s1".into())).unwrap();
        tr.set_node_style(t2, Some("s1".into())).unwrap();
        let (next, ..) = tr.commit();

        let parent_id = next.doc.node(t1).unwrap().parent().unwrap().id();
        let p = next.doc.node(parent_id).unwrap();
        let steps = compact(&p);
        assert_eq!(steps.len(), 1, "same style refs must merge");
    }
}
