use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    match op {
        HistoryOp::Undo => {
            editor.try_undo();
        }
        HistoryOp::Redo => {
            editor.try_redo();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{ChildView, Modifier, ModifierAttrOp, ModifierType};

    use super::*;
    use crate::editor::Editor;

    fn leaf_effective_at(
        editor: &Editor,
        block: editor_crdt::Dot,
        slot: usize,
    ) -> std::collections::BTreeMap<ModifierType, Modifier> {
        let view = editor.state().view();
        let node = view.node(block).unwrap();
        let dot = match node.child_at(slot) {
            Some(ChildView::Leaf(l)) => l.dot(),
            _ => panic!("no leaf at slot {slot}"),
        };
        view.leaf_state_by_dot_slow(dot).unwrap().eff.clone()
    }

    fn carry_tombstone_kinds(
        editor: &Editor,
        block: editor_crdt::Dot,
    ) -> hashbrown::HashSet<ModifierType> {
        editor
            .state()
            .projected
            .node_carries()
            .iter()
            .filter_map(|(_, op)| match op {
                ModifierAttrOp::ClearModifier { target, key } if *target == block => Some(*key),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn deleting_plain_paragraph_carry_tombstones_survive_undo_redo() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("X") } } }
            selection: (p1, 0) -> (p1, 1)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "");
        assert_eq!(carry_tombstone_kinds(&editor, p1).len(), 10);

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "X");
        assert_eq!(carry_tombstone_kinds(&editor, p1).len(), 10);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "");
        assert_eq!(carry_tombstone_kinds(&editor, p1).len(), 10);
    }

    #[test]
    fn undo_of_styled_char_deletion_revives_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") text("X") [bold] text("b") } } }
            selection: (p1, 1) -> (p1, 2)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "ab");

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "aXb");

        assert!(
            leaf_effective_at(&editor, p1, 1).contains_key(&ModifierType::Bold),
            "undo of a styled-char deletion must revive the char's bold paint"
        );
    }

    #[test]
    fn undo_of_partial_unbold_restores_only_the_unbolded_char() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {
                text("가") [bold]
                text("나") [bold]
                text("다") [bold]
            } } }
            selection: (p1, 1) -> (p1, 2)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        assert!(!leaf_effective_at(&editor, p1, 1).contains_key(&ModifierType::Bold));
        assert!(leaf_effective_at(&editor, p1, 0).contains_key(&ModifierType::Bold));
        assert!(leaf_effective_at(&editor, p1, 2).contains_key(&ModifierType::Bold));

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert!(
            leaf_effective_at(&editor, p1, 1).contains_key(&ModifierType::Bold),
            "undo of a partial unbold restores the middle char's bold record"
        );
        assert!(leaf_effective_at(&editor, p1, 0).contains_key(&ModifierType::Bold));
        assert!(leaf_effective_at(&editor, p1, 2).contains_key(&ModifierType::Bold));
    }
}
