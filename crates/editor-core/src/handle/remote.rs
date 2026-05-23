use editor_crdt::Changeset;
use editor_model::DocOp;
use editor_state::StableSelection;
use editor_transaction::StepError;

use crate::editor::Editor;
use crate::error::EditorError;

pub fn handle_remote(editor: &mut Editor, changeset: Changeset<DocOp>) -> Result<(), EditorError> {
    let frozen = editor
        .state
        .selection
        .as_ref()
        .map(|s| StableSelection::freeze(s, &editor.state.doc));
    let (mut next, applied_ops) = editor
        .state
        .receive_remote_changeset(changeset)
        .map_err(|e| EditorError::Step(StepError::State(e)))?;
    next.selection = frozen.map(|f| {
        let thawed = f.thaw(&next.doc);
        thawed.normalize(&next.doc).unwrap_or(thawed)
    });
    editor.state = next;
    editor.pending_ops.extend(applied_ops);
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_crdt::TextOp;
    use editor_macros::state;
    use editor_model::{DocOp, NodeId};
    use editor_state::{DocFlatExt, State};
    use hashbrown::HashSet;

    use crate::editor::Editor;

    #[test]
    fn remote_text_insert_before_cursor_shifts_cursor() {
        let (replica_a, t1) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 1)
        };
        let css_a = replica_a.graph.changesets_as_vec();
        let replica_b = State::from_changesets(css_a, replica_a.selection).unwrap();

        let baseline: HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let (replica_a, _op) = replica_a
            .apply(DocOp::Text {
                node_id: t1,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'X',
                },
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.head.node_id, t1);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn remote_paragraph_delete_relocates_cursor_to_root() {
        let (replica_a, p2, t2) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    p2: paragraph { t2: text("b") }
                }
            }
            selection: (t2, 0)
        };
        let css_a = replica_a.graph.changesets_as_vec();
        let replica_b = State::from_changesets(css_a, replica_a.selection).unwrap();

        use editor_crdt::{OrMapOp, RgaOp};
        let baseline: HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let p2_dot = replica_a
            .doc
            .get_entry(NodeId::ROOT)
            .unwrap()
            .children
            .iter_with_dot()
            .find(|(_, v)| **v == p2)
            .map(|(d, _)| d)
            .unwrap();
        let t2_pres: Vec<_> = replica_a.doc.nodes_tags_for(&t2).copied().collect();
        let p2_pres: Vec<_> = replica_a.doc.nodes_tags_for(&p2).copied().collect();
        let (replica_a, _ops) = replica_a
            .batch_with_ops::<_, editor_state::StateError>(|b| {
                b.apply(DocOp::Presence {
                    node_id: t2,
                    op: OrMapOp::Unset { observed: t2_pres },
                })?;
                b.apply(DocOp::Presence {
                    node_id: p2,
                    op: OrMapOp::Unset { observed: p2_pres },
                })?;
                b.apply(DocOp::Children {
                    node_id: NodeId::ROOT,
                    op: RgaOp::Remove { observed: p2_dot },
                })?;
                Ok(())
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn remote_changeset_applies_when_selection_is_none() {
        let (replica_a, t1) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 1)
        };
        let css_a = replica_a.graph.changesets_as_vec();
        let replica_b = State::from_changesets(css_a, None).unwrap();

        let baseline: HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let (replica_a, _op) = replica_a
            .apply(DocOp::Text {
                node_id: t1,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'X',
                },
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let full_text = editor
            .state()
            .doc
            .flat_text(0..editor.state().doc.flat_size());
        assert!(
            full_text.contains('X'),
            "remote changeset must be applied even when selection is None; doc text: {full_text:?}"
        );
        assert!(
            editor.state().selection.is_none(),
            "selection must remain None when it was None before the remote changeset"
        );
    }

    #[test]
    fn remote_changeset_normalizes_collapsed_on_atom_selection() {
        use editor_crdt::Changeset;
        // replica_a: doc whose first child is an image; initial selection placed at t2.
        let (replica_a, t2) = state! {
            doc { root { image paragraph { t2: text("b") } } }
            selection: (t2, 0)
        };
        let css_a: Vec<Changeset<DocOp>> = replica_a.graph.changesets_as_vec();
        let root = NodeId::ROOT;
        // Seed replica_b with a raw collapsed-on-atom selection (root,0,Down).
        // State::from_changesets does not call normalize, so this abnormal state
        // persists — reproducing the bypass entry point that handle_remote must fix.
        let raw_on_atom = editor_state::Selection::collapsed(editor_state::Position {
            node_id: root,
            offset: 0,
            affinity: editor_state::Affinity::Downstream,
        });
        let replica_b = State::from_changesets(css_a, Some(raw_on_atom)).unwrap();

        // Generate a trivial remote op from replica_a (text insert) to trigger handle_remote.
        let baseline: HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let (replica_a, _op) = replica_a
            .apply(DocOp::Text {
                node_id: t2,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'X',
                },
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        // handle_remote must normalize after thaw, expanding to the image (child[0]) node selection.
        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            !sel.is_collapsed(),
            "remote thaw must normalize collapsed-on-atom, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor,
            editor_state::Position {
                node_id: root,
                offset: 0,
                affinity: editor_state::Affinity::Downstream
            }
        );
        assert_eq!(
            sel.head,
            editor_state::Position {
                node_id: root,
                offset: 1,
                affinity: editor_state::Affinity::Upstream
            }
        );
    }
}
