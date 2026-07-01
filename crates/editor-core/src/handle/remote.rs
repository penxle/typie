use editor_crdt::Changeset;
use editor_model::EditOp;

use crate::editor::Editor;
use crate::error::EditorError;

pub fn handle_remote(editor: &mut Editor, changeset: Changeset<EditOp>) -> Result<(), EditorError> {
    editor.apply_remote_changeset(changeset)
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Changeset, Dot, ListOp};
    use editor_macros::state;
    use editor_model::{EditOp, SeqItem};
    use editor_state::State;
    use hashbrown::HashSet;

    use crate::editor::Editor;

    /// Produce a remote changeset by replaying `ops` onto a clone of `base`'s
    /// projected graph and extracting the resulting local changeset. The
    /// `state!` fixtures all build their graph with actor 1 and deterministic
    /// clocks, so replica_a and replica_b share the same base op identities —
    /// the new op continues replica_a's clock and is unknown to replica_b.
    fn remote_change(base: &State, ops: Vec<EditOp>) -> Changeset<EditOp> {
        let mut pa = base.projected.as_ref().clone();
        let baseline: HashSet<Dot> = pa.graph().current_heads().copied().collect();
        pa.apply_batch(ops).unwrap();
        pa.commit();
        pa.graph()
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0)
    }

    #[test]
    fn remote_text_insert_before_cursor_shifts_cursor() {
        let (replica_a, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css_a, replica_a.selection).unwrap();

        // Insert 'X' at the start of the paragraph ("ab" -> "Xab"); the cursor,
        // anchored after 'a', rebases from offset 1 to offset 2.
        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.head.node, p1);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn remote_paragraph_delete_relocates_cursor_to_root() {
        let (replica_a, _p2) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    p2: paragraph { text("b") }
                }
            }
            selection: (p2, 0)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css_a, replica_a.selection).unwrap();

        // Delete the second paragraph block and its 'b' char (seq positions 2..4:
        // [para0, 'a', p2, 'b']). The cursor, anchored inside the now-dead p2,
        // rebases up to the root.
        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Del { pos: 2, len: 2 })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.head.node, Dot::ROOT);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn remote_changeset_applies_when_selection_is_none() {
        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css_a, None).unwrap();

        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        let view = editor.state().view();
        let full_text = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));
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
    fn probe_remote_changeset_new_op_predicts_true_and_is_safe() {
        use crate::message::Message;
        use crate::test_utils::EditorSnapshot;

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css, replica_a.selection).unwrap();

        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        let before = EditorSnapshot::capture(&editor);
        let probed = editor.can(Message::Remote { changeset: cs }).unwrap();
        let after = EditorSnapshot::capture(&editor);
        assert!(probed, "new remote op must predict true");
        assert_eq!(before, after, "probe must not mutate");
    }

    /// The tick loop coalesces a consecutive run of remote messages into one batched
    /// receive. Applying several remote changesets in a single tick must land the same
    /// document and selection as applying them one tick at a time.
    #[test]
    fn coalesced_remote_batch_matches_sequential() {
        use crate::message::Message;

        let (replica_a, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css = replica_a.graph().changesets_as_vec();

        // Two separate committed remote edits → two changesets building on each other.
        let mut a = replica_a.projected.as_ref().clone();
        let baseline: HashSet<Dot> = a.graph().current_heads().copied().collect();
        a.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Char('X'),
        }))
        .unwrap();
        a.commit();
        a.apply(EditOp::Seq(ListOp::Ins {
            pos: 2,
            item: SeqItem::Char('Y'),
        }))
        .unwrap();
        a.commit();
        let batch = a.graph().local_changesets_since(&baseline).unwrap();
        assert_eq!(batch.len(), 2, "expected two distinct changesets");

        // Batched: both changesets enqueued, drained in one tick (coalesced).
        let mut batched =
            Editor::new_test(State::from_changesets(css.clone(), replica_a.selection).unwrap());
        for cs in &batch {
            batched.enqueue(Message::Remote {
                changeset: cs.clone(),
            });
        }
        let _ = batched.tick().unwrap();

        // Sequential: one changeset per tick.
        let mut sequential =
            Editor::new_test(State::from_changesets(css, replica_a.selection).unwrap());
        for cs in &batch {
            sequential.apply(Message::Remote {
                changeset: cs.clone(),
            });
        }

        let vb = batched.state().view();
        let vs = sequential.state().view();
        let tb = editor_state::flat_text(&vb, 0..editor_state::flat_size(&vb));
        let ts = editor_state::flat_text(&vs, 0..editor_state::flat_size(&vs));
        assert_eq!(tb, ts, "batched and sequential must yield identical text");
        assert!(
            tb.contains('X') && tb.contains('Y'),
            "both edits applied: {tb:?}"
        );
        assert_eq!(
            batched.state().selection,
            sequential.state().selection,
            "selection must match between batched and sequential receive"
        );
    }

    /// Re-receiving an already-applied changeset (a duplicate sync delivery) must be a
    /// no-op: no document change and the selection is left exactly as-is (the tick skips
    /// the selection restore entirely).
    #[test]
    fn duplicate_remote_batch_is_noop() {
        use crate::message::Message;

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css, replica_a.selection).unwrap();

        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.apply(Message::Remote {
            changeset: cs.clone(),
        });
        let after_first = editor.state().selection;
        let view = editor.state().view();
        let text_first = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));

        // Re-receive the same changeset: duplicate, applies nothing.
        editor.apply(Message::Remote {
            changeset: cs.clone(),
        });
        // And a batch of duplicates in one tick.
        editor.enqueue(Message::Remote {
            changeset: cs.clone(),
        });
        editor.enqueue(Message::Remote { changeset: cs });
        let _ = editor.tick().unwrap();

        let view2 = editor.state().view();
        let text_after = editor_state::flat_text(&view2, 0..editor_state::flat_size(&view2));
        assert_eq!(
            text_first, text_after,
            "duplicate receive must not change the doc"
        );
        assert_eq!(
            after_first,
            editor.state().selection,
            "duplicate receive must leave the selection unchanged"
        );
    }

    #[test]
    fn probe_remote_changeset_already_applied_predicts_false() {
        use crate::message::Message;

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let css = replica_a.graph().changesets_as_vec();
        let replica_b = State::from_changesets(css, replica_a.selection).unwrap();

        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.apply(Message::Remote {
            changeset: cs.clone(),
        });
        let probed = editor.can(Message::Remote { changeset: cs }).unwrap();
        assert!(!probed);
    }

    #[test]
    fn remote_changeset_normalizes_collapsed_on_atom_selection() {
        use editor_state::{Affinity, Position, Selection};
        // replica_a: doc whose first child is an image; initial selection placed at p1.
        let (replica_a, _p1) = state! {
            doc { root { image p1: paragraph { text("b") } } }
            selection: (p1, 0)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let root = Dot::ROOT;
        // Seed replica_b with a raw collapsed-on-atom selection (root,0,Down).
        // State::from_changesets does not call normalize, so this abnormal state
        // persists — reproducing the bypass entry point that handle_remote must fix.
        let raw_on_atom = Selection::collapsed(Position {
            node: root,
            offset: 0,
            affinity: Affinity::Downstream,
        });
        let replica_b = State::from_changesets(css_a, Some(raw_on_atom)).unwrap();

        // Generate a trivial remote op (insert 'X' before 'b', seq pos 2:
        // [image, para, 'b']) to trigger handle_remote.
        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Char('X'),
            })],
        );

        let mut editor = Editor::new_test(replica_b);
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        // handle_remote must normalize after restore, expanding to the image (child[0]) node selection.
        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            !sel.is_collapsed(),
            "remote restore must normalize collapsed-on-atom, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor,
            Position {
                node: root,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node: root,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
    }

    /// Regression: a remote op that shrinks the node a local undo entry's caret
    /// was recorded in must not leave the caret dangling after undo.
    ///
    /// Replica B records an undo entry while its caret sits at offset 2 of `p1`
    /// ("xy"). A concurrent remote op deletes both base chars, so after the merge
    /// `p1` holds only B's own insert. Undoing that insert empties `p1` — the
    /// recorded offset 2 no longer exists. Restoring the caret as a raw position
    /// would dangle (the selection stops resolving, which freezes further editing
    /// — the reported "selection goes weird, then undo/redo stop working"). The
    /// StableSelection round-trip must instead land on a valid position.
    #[test]
    fn undo_after_remote_shrinks_caret_host_keeps_selection_resolvable() {
        use crate::message::{HistoryOp, InsertionOp, Message};

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("xy") } } }
            selection: (p1, 2)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        // Distinct actor for B (from_changesets), so its local insert cannot
        // collide with the actor-1 remote op.
        let replica_b = State::from_changesets(css_a, replica_a.selection).unwrap();
        let mut editor = Editor::new_test(replica_b);

        // B types 'z' at the end of p1 → one undo entry whose recorded caret is (p1, 2).
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "z".into() },
        });

        // Remote deletes the two base chars "xy" (seq flat: [block, 'x', 'y']).
        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Del { pos: 1, len: 2 })],
        );
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        // Undo B's insert: p1 becomes empty, so the recorded offset 2 is gone.
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        let sel = editor
            .state()
            .selection
            .expect("selection present after undo");
        assert!(
            sel.resolve(&editor.state().view()).is_some(),
            "undo must re-resolve the recorded caret against the restructured doc, got {sel:?}",
        );

        // The editor must not be frozen: a subsequent edit still applies (this
        // would panic via `tick().unwrap()` if the caret were dangling).
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "w".into() },
        });
        let after = editor.state();
        let view = after.view();
        assert!(
            after.selection.and_then(|s| s.resolve(&view)).is_some(),
            "editor must remain editable after undo",
        );
    }
}
