use editor_crdt::Changeset;
use editor_model::DocOp;
use editor_state::StableSelection;
use editor_transaction::StepError;

use crate::editor::Editor;
use crate::error::EditorError;

pub fn handle_remote(editor: &mut Editor, changeset: Changeset<DocOp>) -> Result<(), EditorError> {
    let frozen = StableSelection::freeze(&editor.state.selection, &editor.state.doc);
    let (mut next, applied_ops) = editor
        .state
        .receive_remote_changeset(changeset)
        .map_err(|e| EditorError::Step(StepError::State(e)))?;
    next.selection = frozen.thaw(&next.doc);
    editor.state = next;
    editor.pending_ops.extend(applied_ops);
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_crdt::TextOp;
    use editor_macros::state;
    use editor_model::{DocOp, NodeId};
    use editor_state::State;
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

        assert_eq!(editor.state().selection.head.node_id, t1);
        assert_eq!(editor.state().selection.head.offset, 2);
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

        assert_eq!(editor.state().selection.head.node_id, NodeId::ROOT);
        assert_eq!(editor.state().selection.head.offset, 1);
    }
}
