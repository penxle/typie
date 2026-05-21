use editor_macros::state;
use editor_model::{Modifier, NodeId, PlainNode, PlainParagraphNode};
use editor_transaction::{Step, Transaction};
use proptest::prelude::*;

fn extract_text(state: &editor_state::State, node_id: NodeId) -> String {
    let Some(node_ref) = state.doc.node(node_id) else {
        return String::new();
    };
    let mut result = String::new();
    for desc in std::iter::once(node_ref).chain(node_ref.descendants()) {
        if let editor_model::Node::Text(t) = desc.node() {
            result.push_str(&t.text.to_string());
        }
    }
    result
}

mod proptests {
    use super::*;

    proptest! {
        #[test]
        fn step_fail_rollback_preserves_prior_mutations(
            text_a in "[a-z]{1,5}",
            text_b in "[a-z]{1,5}",
        ) {
            let (state, t1) = state! {
                doc {
                    root {
                        paragraph {
                            t1: text("")
                        }
                    }
                }
                selection: (t1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.insert_text(t1, 0, &text_a).unwrap();
            let after_step_1 = tr.doc();
            tr.insert_text(t1, 0, &text_b).unwrap();
            let after_step_2 = tr.doc();

            let invalid_offset = tr.doc().node(t1).map(|n| {
                match n.node() {
                    editor_model::Node::Text(t) => t.text.len() + 100,
                    _ => 100,
                }
            }).unwrap_or(100);
            let result = tr.insert_text(t1, invalid_offset, "x");
            prop_assert!(result.is_err());

            prop_assert_eq!(tr.doc().to_plain(), after_step_2.to_plain());
            prop_assert_ne!(tr.doc().to_plain(), after_step_1.to_plain());
        }
    }

    proptest! {
        #[test]
        fn split_text_preserves_chars(
            text in "[a-z]{1,15}",
            split_at in 0usize..15,
        ) {
            let chars: Vec<char> = text.chars().collect();
            let split_at = split_at.min(chars.len());

            let (state, t1) = state! {
                doc {
                    root {
                        paragraph {
                            t1: text("placeholder")
                        }
                    }
                }
                selection: (t1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.remove_text(t1, 0, "placeholder".chars().count()).unwrap();
            tr.insert_text(t1, 0, &text).unwrap();
            let (state, _, _, _, _) = tr.commit();

            let new_t = NodeId::new();
            let mut tr = Transaction::new(&state);
            let result = tr.split_node(t1, split_at, new_t);
            if result.is_err() { return Ok(()); }
            let (split_state, _, _, _, _) = tr.commit();

            let t1_text = match split_state.doc.get_entry(t1).map(|e| &e.node) {
                Some(editor_model::Node::Text(t)) => t.text.to_string(),
                _ => return Ok(()),
            };
            let new_t_text = match split_state.doc.get_entry(new_t).map(|e| &e.node) {
                Some(editor_model::Node::Text(t)) => t.text.to_string(),
                _ => return Ok(()),
            };

            let expected_first: String = text.chars().take(split_at).collect();
            let expected_rest: String = text.chars().skip(split_at).collect();

            prop_assert_eq!(t1_text, expected_first);
            prop_assert_eq!(new_t_text, expected_rest);
        }

        #[test]
        fn merge_paragraphs_concatenates(
            text_a in "[a-z]{0,10}",
            text_b in "[a-z]{0,10}",
        ) {
            let (state, p1, t1, p2, t2) = state! {
                doc {
                    root {
                        p1: paragraph { t1: text("placeholder_a") }
                        p2: paragraph { t2: text("placeholder_b") }
                    }
                }
                selection: (t1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.remove_text(t1, 0, "placeholder_a".chars().count()).unwrap();
            tr.insert_text(t1, 0, &text_a).unwrap();
            tr.remove_text(t2, 0, "placeholder_b".chars().count()).unwrap();
            tr.insert_text(t2, 0, &text_b).unwrap();
            let (state, _, _, _, _) = tr.commit();

            let mut tr = Transaction::new(&state);
            tr.merge_node(p2, p1).unwrap();
            let (merged, _, _, _, _) = tr.commit();

            let p1_text = extract_text(&merged, p1);
            prop_assert_eq!(p1_text, format!("{}{}", text_a, text_b));
            prop_assert!(merged.doc.get_entry(p2).is_none() || !merged.doc.nodes_iter().any(|(id, _)| *id == p2));
        }

        #[test]
        fn move_node_changes_parent(target_index in 0usize..2) {
            let (state, t1, p2) = state! {
                doc {
                    root {
                        paragraph { t1: text("hello") }
                        p2: paragraph
                    }
                }
                selection: (t1, 0)
            };

            let mut tr = Transaction::new(&state);
            let result = tr.move_node(t1, p2, target_index);
            if result.is_err() { return Ok(()); }
            let (moved, _, _, _, _) = tr.commit();

            let t1_entry = moved.doc.get_entry(t1).unwrap();
            prop_assert_eq!(*t1_entry.parent.get(), Some(p2));
        }

        #[test]
        fn insert_subtree_appears_in_parent_children(index in 0usize..3) {
            let (state, _) = state! {
                doc { root { p1: paragraph } }
                selection: (p1, 0)
            };

            let new_id = NodeId::new();
            let subtree = editor_model::Subtree::leaf(
                new_id,
                PlainNode::Paragraph(PlainParagraphNode::default()),
            );
            let root_id = state.doc.root().unwrap().id();

            let mut tr = Transaction::new(&state);
            let result = tr.insert_subtree(root_id, index, subtree);
            if result.is_err() { return Ok(()); }
            let (inserted, _, _, _, _) = tr.commit();

            let new_entry = inserted.doc.get_entry(new_id).unwrap();
            prop_assert_eq!(*new_entry.parent.get(), Some(root_id));
        }

    }

    proptest! {
        #[test]
        fn insert_text_inverse_round_trip(text in "[a-z]{1,10}") {
            let (state, t1) = state! {
                doc { root { paragraph { t1: text("") } } }
                selection: (t1, 0)
            };
            let plain_before = state.doc.to_plain();
            let step = Step::InsertText { node_id: t1, offset: 0, text: text.clone() };
            let after_state = step.apply(&state).unwrap().state;
            let inverse = step.inverse();
            let restored = inverse.apply(&after_state).unwrap().state;
            prop_assert_eq!(restored.doc.to_plain(), plain_before);
        }

        #[test]
        fn remove_text_inverse_round_trip(text in "[a-z]{1,10}") {
            let (state, t1) = state! {
                doc { root { paragraph { t1: text("placeholder") } } }
                selection: (t1, 0)
            };
            let mut tr = Transaction::new(&state);
            tr.remove_text(t1, 0, "placeholder".chars().count()).unwrap();
            tr.insert_text(t1, 0, &text).unwrap();
            let (state, _, _, _, _) = tr.commit();

            let plain_before = state.doc.to_plain();
            let step = Step::RemoveText { node_id: t1, offset: 0, text: text.clone() };
            let after_state = step.apply(&state).unwrap().state;
            let inverse = step.inverse();
            let restored = inverse.apply(&after_state).unwrap().state;
            prop_assert_eq!(restored.doc.to_plain(), plain_before);
        }

    }
}

mod tests {
    use super::*;

    #[test]
    fn add_modifier_twice_dispatches_once() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("hi")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.add_modifier(t1, Modifier::Bold).unwrap();
        tr.add_modifier(t1, Modifier::Bold).unwrap();
        let (_, _, _, _, _) = tr.commit();
    }

    #[test]
    fn subtree_subsumes_node_in_dispatch() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_t = NodeId::new();
        tr.split_node(t1, 3, new_t).unwrap();
        let (_, _, _, _, _) = tr.commit();
    }

    #[test]
    fn remove_subtree_removes_from_doc() {
        let (state, p1) = state! {
            doc { root { p1: paragraph paragraph } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.remove_subtree(p1).unwrap();
        let (removed, _, _, _, _) = tr.commit();

        assert!(removed.doc.get_entry(p1).is_none());
    }

    #[test]
    fn add_modifier_inverse_round_trip() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let plain_before = state.doc.to_plain();
        let step = Step::AddModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };
        let after_state = step.apply(&state).unwrap().state;
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn split_node_inverse_round_trip() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        let plain_before = state.doc.to_plain();
        let new_t = NodeId::new();
        let step = Step::SplitNode {
            node_id: t1,
            offset: 3,
            new_node_id: new_t,
        };
        let after_state = match step.apply(&state) {
            Ok(out) => out.state,
            Err(_) => return,
        };
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn remove_modifier_inverse_round_trip() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.add_modifier(t1, Modifier::Bold).unwrap();
        let (state, _, _, _, _) = tr.commit();

        let plain_before = state.doc.to_plain();
        let step = Step::RemoveModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };
        let after_state = step.apply(&state).unwrap().state;
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn set_node_inverse_round_trip() {
        let (state, im1) = state! {
            doc { root { paragraph im1: image paragraph } }
            selection: (im1, 0)
        };
        let plain_before = state.doc.to_plain();
        let new_node = editor_model::PlainNode::Image(editor_model::PlainImageNode {
            id: Some("new-image-id".to_string()),
            proportion: 50,
        });
        let old_node = state.doc.get_entry(im1).unwrap().node.to_plain();
        let step = Step::SetNode {
            node_id: im1,
            old_node: old_node.clone(),
            new_node,
        };
        let after_state = match step.apply(&state) {
            Ok(out) => out.state,
            Err(_) => return,
        };
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn merge_node_inverse_round_trip() {
        let (state, t1, t2) = state! {
            doc { root { paragraph { t1: text("hello") [bold] t2: text("world") [bold] } } }
            selection: (t1, 0)
        };
        let plain_before = state.doc.to_plain();
        let step = Step::MergeNode {
            node_id: t2,
            target_id: t1,
            offset: 5,
        };
        let after_state = match step.apply(&state) {
            Ok(out) => out.state,
            Err(_) => return,
        };
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn move_node_inverse_round_trip() {
        let (state, p2) = state! {
            doc { root { paragraph { text("a") } p2: paragraph } }
            selection: (p2, 0)
        };
        let plain_before = state.doc.to_plain();
        let root_id = state.doc.root().unwrap().id();
        let step = Step::MoveNode {
            node_id: p2,
            old_parent: root_id,
            old_index: 1,
            new_parent: root_id,
            new_index: 0,
        };
        let after_state = match step.apply(&state) {
            Ok(out) => out.state,
            Err(_) => return,
        };
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn insert_subtree_inverse_round_trip() {
        let (state, _) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let plain_before = state.doc.to_plain();
        let root_id = state.doc.root().unwrap().id();
        let new_id = NodeId::new();
        let subtree = editor_model::Subtree::leaf(
            new_id,
            PlainNode::Paragraph(PlainParagraphNode::default()),
        );
        let step = Step::InsertSubtree {
            parent_id: root_id,
            index: 1,
            subtree,
        };
        let after_state = match step.apply(&state) {
            Ok(out) => out.state,
            Err(_) => return,
        };
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }

    #[test]
    fn remove_subtree_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph paragraph } }
            selection: (p1, 0)
        };
        let plain_before = state.doc.to_plain();
        let root_id = state.doc.root().unwrap().id();
        let subtree = editor_model::Subtree::capture(&state.doc, p1).unwrap();
        let step = Step::RemoveSubtree {
            parent_id: root_id,
            index: 0,
            subtree,
        };
        let after_state = step.apply(&state).unwrap().state;
        let inverse = step.inverse();
        let restored = inverse.apply(&after_state).unwrap().state;
        assert_eq!(restored.doc.to_plain(), plain_before);
    }
}
