use editor_crdt::EntryDot;
use editor_transaction::{
    HistoryMeta, Step, StepError, StepRecord, TextInsertEffect, TextRemoveEffect, Transaction,
};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::history::HistoryPlaybackStep;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    let (playback, is_redo) = match op {
        HistoryOp::Undo => (editor.try_undo(), false),
        HistoryOp::Redo => (editor.try_redo(), true),
    };

    if let Some(playback) = playback {
        editor.transact(|tr| {
            apply_history_playback(tr, &playback)?;
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
    steps: &[HistoryPlaybackStep],
) -> Result<bool, StepError> {
    tr.update_meta(|m| m.history = HistoryMeta::Skip);

    let doc_playback_steps = history_playback_doc_steps(steps);
    let doc_steps = doc_playback_steps
        .iter()
        .map(|step| step.step_to_apply.clone())
        .collect();
    let state_steps = history_playback_state_steps(steps);
    let playback_steps = tr.apply_steps(doc_steps)?;
    apply_stable_position_remaps_for_playback(tr, &doc_playback_steps, &playback_steps)?;
    // Stable selections in local state steps must restore after remaps from the
    // freshly replayed doc entries have been installed.
    tr.apply_steps(state_steps)?;

    Ok(!steps.is_empty())
}

fn history_playback_doc_steps(steps: &[HistoryPlaybackStep]) -> Vec<&HistoryPlaybackStep> {
    steps
        .iter()
        .filter(|step| step.step_to_apply.is_doc_step())
        .collect()
}

fn history_playback_state_steps(steps: &[HistoryPlaybackStep]) -> Vec<Step> {
    steps
        .iter()
        .filter(|step| !step.step_to_apply.is_doc_step())
        .map(|step| step.step_to_apply.clone())
        .collect()
}

fn apply_stable_position_remaps_for_playback(
    tr: &mut Transaction,
    source: &[&HistoryPlaybackStep],
    playback_steps: &[StepRecord],
) -> Result<(), StepError> {
    for idx in 0..source.len() {
        let source_step = source[idx];
        let playback_step = &playback_steps[idx];
        if !has_text_effect(&source_step.source) || !has_text_effect(playback_step) {
            continue;
        }
        apply_remove_to_insert_remaps(
            tr,
            source_step.source.effect.text_removes.iter(),
            playback_step.effect.text_inserts.iter(),
        )?;
        apply_insert_effect_remaps(
            tr,
            source_step.source.effect.text_inserts.iter(),
            playback_step.effect.text_inserts.iter(),
        )?;
    }
    Ok(())
}

fn has_text_effect(record: &StepRecord) -> bool {
    !record.effect.text_inserts.is_empty() || !record.effect.text_removes.is_empty()
}

fn apply_remove_to_insert_remaps<'a>(
    tr: &mut Transaction,
    from_effects: impl ExactSizeIterator<Item = &'a TextRemoveEffect>,
    to_effects: impl ExactSizeIterator<Item = &'a TextInsertEffect>,
) -> Result<(), StepError> {
    if from_effects.len() != to_effects.len() {
        return Ok(());
    }

    for (from, to) in from_effects.zip(to_effects) {
        apply_entry_remaps(tr, &from.entries, &from.text, &to.entries, &to.text)?;
    }
    Ok(())
}

pub(super) fn apply_insert_effect_remaps<'a>(
    tr: &mut Transaction,
    from_effects: impl ExactSizeIterator<Item = &'a TextInsertEffect>,
    to_effects: impl ExactSizeIterator<Item = &'a TextInsertEffect>,
) -> Result<(), StepError> {
    // HistoryPlaybackStep pairs the semantic source step with the step applied
    // during undo/redo. Text effects are still emitted independently by those
    // applications; if their shape differs, entry mapping is ambiguous.
    if from_effects.len() != to_effects.len() {
        return Ok(());
    }

    for (from, to) in from_effects.zip(to_effects) {
        apply_entry_remaps(tr, &from.entries, &from.text, &to.entries, &to.text)?;
    }
    Ok(())
}

pub(super) fn apply_entry_remaps(
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
    use editor_transaction::{
        HistoryMeta, Step, StepEffect, StepRecord, TextInsertEffect, Transaction,
    };

    use super::*;
    use crate::history::HistoryPlaybackStep;

    fn playback_step(source: StepRecord, step_to_apply: Step) -> HistoryPlaybackStep {
        HistoryPlaybackStep {
            source,
            step_to_apply,
        }
    }

    fn empty_record(step: Step) -> StepRecord {
        StepRecord {
            step,
            effect: StepEffect::default(),
        }
    }

    #[test]
    fn applies_doc_steps_and_marks_history_skip() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("a") } } }
            selection: (t, 1)
        };
        let mut tr = Transaction::new(&state);
        let step = Step::InsertText {
            node_id: t,
            offset: 1,
            text: "x".into(),
        };

        let changed =
            apply_history_playback(&mut tr, &[playback_step(empty_record(step.clone()), step)])
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
            &[playback_step(source_step.clone(), source_step.step.clone())],
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

        let step = Step::InsertText {
            node_id: t,
            offset: 1,
            text: "y".into(),
        };
        apply_history_playback(&mut tr, &[playback_step(source_step, step)]).unwrap();

        assert_eq!(
            tr.doc().text_identity().resolve_stable_entry(old_x),
            StableEntryResolution::Deleted(old_x)
        );
    }

    #[test]
    fn skips_insert_remaps_when_effect_counts_differ() {
        let (state, t, old_a, old_b, source_effects) = state_after_two_inserts_then_remove();
        let mut tr = Transaction::new(&state);

        tr.insert_text(t, 0, "a").unwrap();
        let replacement_effects: Vec<TextInsertEffect> = tr
            .step_records_since(0)
            .iter()
            .flat_map(|record| record.effect.text_inserts.iter().cloned())
            .collect();

        apply_insert_effect_remaps(&mut tr, source_effects.iter(), replacement_effects.iter())
            .unwrap();

        assert_eq!(
            tr.doc().text_identity().resolve_stable_entry(old_a),
            StableEntryResolution::Deleted(old_a)
        );
        assert_eq!(
            tr.doc().text_identity().resolve_stable_entry(old_b),
            StableEntryResolution::Deleted(old_b)
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

    fn state_after_two_inserts_then_remove() -> (
        editor_state::State,
        NodeId,
        EntryDot,
        EntryDot,
        Vec<TextInsertEffect>,
    ) {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("") } } }
            selection: (t, 0)
        };

        let mut insert_a_tr = Transaction::new(&initial);
        insert_a_tr.insert_text(t, 0, "a").unwrap();
        let (with_a, insert_a_records, ..) = insert_a_tr.commit();
        let insert_a_effect = insert_a_records[0].effect.text_inserts[0].clone();
        let old_a = insert_a_effect.entries[0];

        let mut insert_b_tr = Transaction::new(&with_a);
        insert_b_tr.insert_text(t, 1, "b").unwrap();
        let (with_ab, insert_b_records, ..) = insert_b_tr.commit();
        let insert_b_effect = insert_b_records[0].effect.text_inserts[0].clone();
        let old_b = insert_b_effect.entries[0];

        let mut remove_tr = Transaction::new(&with_ab);
        remove_tr.remove_text(t, 0, 2).unwrap();
        let (removed, ..) = remove_tr.commit();
        assert_eq!(removed.doc.text_view(t).unwrap().text(), "");

        (
            removed,
            t,
            old_a,
            old_b,
            vec![insert_a_effect, insert_b_effect],
        )
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
