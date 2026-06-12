use editor_crdt::EntryDot;
use editor_transaction::{
    HistoryMeta, Step, StepError, StepRecord, TextInsertEffect, TextRemoveEffect, Transaction,
};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    let (playback, is_redo) = match op {
        HistoryOp::Undo => (editor.try_undo(), false),
        HistoryOp::Redo => (editor.try_redo(), true),
    };

    if let Some(playback) = playback {
        editor.transact(|tr| {
            apply_history_playback(tr, &playback.steps_to_apply, &playback.source_steps)?;
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

pub(super) fn apply_history_playback(
    tr: &mut Transaction,
    steps_to_apply: &[Step],
    source_steps: &[StepRecord],
) -> Result<bool, StepError> {
    tr.update_meta(|m| m.history = HistoryMeta::Skip);

    let doc_steps = history_playback_doc_steps(steps_to_apply);
    let state_steps = history_playback_state_steps(steps_to_apply);
    let playback_steps = tr.apply_steps(doc_steps)?;
    apply_stable_position_remaps_for_playback(tr, source_steps, &playback_steps)?;
    // Stable selections in local state steps must thaw after remaps from the
    // freshly replayed doc entries have been installed.
    tr.apply_steps(state_steps)?;

    Ok(!steps_to_apply.is_empty())
}

fn history_playback_doc_steps(steps: &[Step]) -> Vec<Step> {
    steps
        .iter()
        .filter(|step| step.is_doc_step())
        .cloned()
        .collect()
}

fn history_playback_state_steps(steps: &[Step]) -> Vec<Step> {
    steps
        .iter()
        .filter(|step| !step.is_doc_step())
        .cloned()
        .collect()
}

fn apply_stable_position_remaps_for_playback(
    tr: &mut Transaction,
    source: &[StepRecord],
    playback_steps: &[StepRecord],
) -> Result<(), StepError> {
    let source_text_steps = source.iter().filter(|record| has_text_effect(record));
    let playback_text_steps = playback_steps
        .iter()
        .filter(|record| has_text_effect(record));
    // TODO: history playback step을 pair로 들고 zip을 없앤다.
    for (source_step, playback_step) in source_text_steps.zip(playback_text_steps) {
        apply_remove_to_insert_remaps(
            tr,
            &source_step.effect.text_removes,
            &playback_step.effect.text_inserts,
        )?;
        apply_insert_to_insert_remaps(
            tr,
            &source_step.effect.text_inserts,
            &playback_step.effect.text_inserts,
        )?;
    }
    Ok(())
}

fn has_text_effect(record: &StepRecord) -> bool {
    !record.effect.text_inserts.is_empty() || !record.effect.text_removes.is_empty()
}

fn apply_remove_to_insert_remaps(
    tr: &mut Transaction,
    from_effects: &[TextRemoveEffect],
    to_effects: &[TextInsertEffect],
) -> Result<(), StepError> {
    for (from, to) in from_effects.iter().zip(to_effects.iter()) {
        apply_entry_remaps(tr, &from.entries, &from.text, &to.entries, &to.text)?;
    }
    Ok(())
}

fn apply_insert_to_insert_remaps(
    tr: &mut Transaction,
    from_effects: &[TextInsertEffect],
    to_effects: &[TextInsertEffect],
) -> Result<(), StepError> {
    for (from, to) in from_effects.iter().zip(to_effects.iter()) {
        apply_entry_remaps(tr, &from.entries, &from.text, &to.entries, &to.text)?;
    }
    Ok(())
}

fn apply_entry_remaps(
    tr: &mut Transaction,
    from_entries: &[EntryDot],
    from_text: &str,
    to_entries: &[EntryDot],
    to_text: &str,
) -> Result<(), StepError> {
    if from_text != to_text || from_entries.len() != to_entries.len() {
        return Ok(());
    }

    for (from, to) in from_entries.iter().copied().zip(to_entries.iter().copied()) {
        if from != to {
            tr.stable_position_remap(from, to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests_playback {
    use editor_crdt::EntryDot;
    use editor_macros::state;
    use editor_model::{NodeId, StableEntryResolution};
    use editor_transaction::{HistoryMeta, Step, StepRecord, Transaction};

    use super::*;

    #[test]
    fn applies_doc_steps_and_marks_history_skip() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("a") } } }
            selection: (t, 1)
        };
        let mut tr = Transaction::new(&state);

        let changed = apply_history_playback(
            &mut tr,
            &[Step::InsertText {
                node_id: t,
                offset: 1,
                text: "x".into(),
            }],
            &[],
        )
        .unwrap();

        assert!(changed);
        assert!(matches!(tr.meta().history, HistoryMeta::Skip));
        assert_eq!(tr.doc().text_view(t).unwrap().text(), "ax");
    }

    #[test]
    fn remaps_source_insert_entries_to_fresh_playback_entries() {
        let (state, t, source_step, old_x) = state_after_insert_then_remove();
        let mut tr = Transaction::new(&state);

        apply_history_playback(
            &mut tr,
            std::slice::from_ref(&source_step.step),
            std::slice::from_ref(&source_step),
        )
        .unwrap();

        let fresh_x = tr
            .doc()
            .text_view(t)
            .unwrap()
            .visible_entries()
            .nth(1)
            .unwrap()
            .0;
        assert_eq!(
            tr.doc().text_identity().resolve_stable_entry(old_x),
            StableEntryResolution::Live(fresh_x)
        );
    }

    #[test]
    fn ignores_mismatched_text_effects() {
        let (state, t, source_step, old_x) = state_after_insert_then_remove();
        let mut tr = Transaction::new(&state);

        apply_history_playback(
            &mut tr,
            &[Step::InsertText {
                node_id: t,
                offset: 1,
                text: "y".into(),
            }],
            std::slice::from_ref(&source_step),
        )
        .unwrap();

        assert_eq!(
            tr.doc().text_identity().resolve_stable_entry(old_x),
            StableEntryResolution::Deleted(old_x)
        );
    }

    fn state_after_insert_then_remove() -> (editor_state::State, NodeId, StepRecord, EntryDot) {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("a") } } }
            selection: (t, 1)
        };
        let mut insert_tr = Transaction::new(&initial);
        insert_tr.insert_text(t, 1, "x").unwrap();
        let (inserted, insert_records, ..) = insert_tr.commit();
        let insert_record = insert_records.into_iter().next().unwrap();
        let old_x = insert_record.effect.text_inserts[0].entries[0];

        let mut remove_tr = Transaction::new(&inserted);
        remove_tr.remove_text(t, 1, 1).unwrap();
        let (removed, ..) = remove_tr.commit();
        assert_eq!(removed.doc.text_view(t).unwrap().text(), "a");

        (removed, t, insert_record, old_x)
    }
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
