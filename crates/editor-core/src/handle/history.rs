use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    let (steps, is_redo) = match op {
        HistoryOp::Undo => (editor.try_undo(), false),
        HistoryOp::Redo => (editor.try_redo(), true),
    };

    if let Some(steps) = steps {
        editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            tr.apply_steps(steps.to_vec())?;
            Ok(())
        })?;
        // Redo re-applies the original tagged entry. Restore last_tag so that
        // shortcut gestures (e.g. backspace after auto-replacement) still fire.
        if is_redo {
            editor.sync_history_last_tag_from_top();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests_probe {
    use editor_macros::state;

    use crate::editor::Editor;
    use crate::message::*;
    use crate::test_utils::{assert_probe_predicts_apply, assert_probe_predicts_apply_with_setup};

    #[test]
    fn probe_undo_empty_history() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::History {
                op: HistoryOp::Undo,
            },
        );
    }

    #[test]
    fn probe_undo_after_insertion_with_history_preserved() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_probe_predicts_apply_with_setup(
            || {
                let mut editor = Editor::new_test(state.clone());
                editor.apply(Message::Insertion {
                    op: InsertionOp::Text {
                        text: " world".into(),
                    },
                });
                editor
            },
            Message::History {
                op: HistoryOp::Undo,
            },
        );
    }

    #[test]
    fn probe_redo_empty() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::History {
                op: HistoryOp::Redo,
            },
        );
    }

    #[test]
    fn probe_redo_after_undo_with_history_preserved() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_probe_predicts_apply_with_setup(
            || {
                let mut editor = Editor::new_test(state.clone());
                editor.apply(Message::Insertion {
                    op: InsertionOp::Text {
                        text: " world".into(),
                    },
                });
                editor.apply(Message::History {
                    op: HistoryOp::Undo,
                });
                editor
            },
            Message::History {
                op: HistoryOp::Redo,
            },
        );
    }
}
