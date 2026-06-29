use editor_commands::{self as commands};
use editor_state::StableSelection;
use editor_view::GroupDecoration;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::{EditorEvent, TrackedRangeReplaceOutcome};
use crate::message::*;
use crate::state_field::StateField;
use crate::tracked_range::TrackedRange;

pub fn handle_tracked_range_op(editor: &mut Editor, op: TrackedRangeOp) -> Result<(), EditorError> {
    match op {
        TrackedRangeOp::Add {
            id,
            group,
            selection,
            metadata,
        } => {
            let view = editor.state().view();
            if selection.anchor.resolve(&view).is_none() || selection.head.resolve(&view).is_none()
            {
                return Err(EditorError::General {
                    msg: "TrackedRange::Add: selection must resolve against current doc".into(),
                });
            }
            let stable = StableSelection::capture(&selection, &view);
            let new_range = TrackedRange::new(id.clone(), group, stable, metadata, editor.state());
            let would_change = editor
                .tracked_ranges()
                .get(&id)
                .map(|existing| existing != &new_range)
                .unwrap_or(true);
            commit_or_probe(editor, would_change, |reg| {
                reg.add(new_range);
            });
        }
        TrackedRangeOp::AddFrozen {
            id,
            group,
            selection,
            metadata,
        } => {
            let new_range =
                TrackedRange::new(id.clone(), group, selection, metadata, editor.state());
            let would_change = editor
                .tracked_ranges()
                .get(&id)
                .map(|existing| existing != &new_range)
                .unwrap_or(true);
            commit_or_probe(editor, would_change, |reg| {
                reg.add(new_range);
            });
        }
        TrackedRangeOp::Remove { id } => {
            let would_change = editor.tracked_ranges().contains(&id);
            commit_or_probe(editor, would_change, |reg| {
                reg.remove(&id);
            });
        }
        TrackedRangeOp::SetGroup { id, group } => {
            // Re-anchor the range to its current resolved extent so a stale
            // binding to a since-deleted boundary character is dropped; without
            // this, a later insert at the collapsed boundary would be recaptured.
            let recaptured = editor.tracked_ranges().get(&id).and_then(|range| {
                let state = editor.state();
                let located = range.locate(state)?;
                let view = state.view();
                Some(StableSelection::capture(&located, &view))
            });
            let would_change = editor.tracked_ranges().get(&id).is_some_and(|range| {
                range.group != group || recaptured.as_ref().is_some_and(|s| s != &range.selection)
            });
            commit_or_probe(editor, would_change, |reg| {
                if let Some(selection) = recaptured {
                    reg.set_selection(&id, selection);
                }
                reg.set_group(&id, group);
            });
        }
        TrackedRangeOp::ClearGroup { group } => {
            let would_change = editor.tracked_ranges().group_size(&group) > 0;
            commit_or_probe(editor, would_change, |reg| {
                reg.clear_group(&group);
            });
        }
        TrackedRangeOp::Invalidate { id } => {
            let would_change = editor
                .tracked_ranges()
                .get(&id)
                .is_some_and(|r| !r.explicitly_invalid);
            commit_or_probe(editor, would_change, |reg| {
                reg.invalidate(&id);
            });
        }
        TrackedRangeOp::SetGroupDecoration {
            group,
            style,
            enabled,
            z_index,
        } => {
            let decoration = GroupDecoration {
                style,
                enabled,
                z_index,
            };
            let would_change = editor.view.would_set_group_decoration(&group, &decoration);
            commit_view_or_probe(editor, would_change, |editor| {
                editor.view.set_group_decoration(group, decoration);
            });
        }
        TrackedRangeOp::RemoveGroupDecoration { group } => {
            let would_change = editor.view.would_remove_group_decoration(&group);
            commit_view_or_probe(editor, would_change, |editor| {
                editor.view.remove_group_decoration(&group);
            });
        }
        TrackedRangeOp::ReplaceText {
            id,
            expected_text,
            replacement,
        } => {
            handle_replace_text(editor, id, expected_text, replacement)?;
        }
    }
    Ok(())
}

fn handle_replace_text(
    editor: &mut Editor,
    id: String,
    expected_text: Option<String>,
    replacement: String,
) -> Result<(), EditorError> {
    let classification = classify_replace_text(editor, &id, expected_text.as_deref(), &replacement);

    if editor.is_probing() {
        editor.mark_probed_change(classification.selection.is_some());
        return Ok(());
    }

    if let Some(selection) = classification.selection {
        editor.transact(|tr| {
            commands::replace_tracked_range(tr, selection, &replacement)?;
            Ok(())
        })?;
    }

    editor.push_event(EditorEvent::TrackedRangeReplaceResult {
        id,
        outcome: classification.outcome,
    });
    Ok(())
}

struct ReplaceTextClassification {
    outcome: TrackedRangeReplaceOutcome,
    selection: Option<editor_state::Selection>,
}

fn classify_replace_text(
    editor: &Editor,
    id: &str,
    expected_text: Option<&str>,
    replacement: &str,
) -> ReplaceTextClassification {
    let Some(range) = editor.tracked_ranges().get(id) else {
        return ReplaceTextClassification {
            outcome: TrackedRangeReplaceOutcome::UnknownId,
            selection: None,
        };
    };
    let Some(selection) = range.locate(&editor.state) else {
        return ReplaceTextClassification {
            outcome: TrackedRangeReplaceOutcome::Invalid,
            selection: None,
        };
    };
    if replacement.contains(['\n', '\r']) {
        return ReplaceTextClassification {
            outcome: TrackedRangeReplaceOutcome::InvalidReplacement,
            selection: None,
        };
    }
    if let Some(expected) = expected_text {
        use editor_state::ResolvedPositionFlatExt;
        let view = editor.state.view();
        let Some(resolved) = selection.resolve(&view) else {
            return ReplaceTextClassification {
                outcome: TrackedRangeReplaceOutcome::Invalid,
                selection: None,
            };
        };
        let range = resolved.from().to_flat()..resolved.to().to_flat();
        if editor_state::flat_text(&view, range) != expected {
            return ReplaceTextClassification {
                outcome: TrackedRangeReplaceOutcome::TextMismatch,
                selection: None,
            };
        }
    }
    ReplaceTextClassification {
        outcome: TrackedRangeReplaceOutcome::Replaced,
        selection: Some(selection),
    }
}

fn commit_view_or_probe<F>(editor: &mut Editor, would_change: bool, apply: F)
where
    F: FnOnce(&mut Editor),
{
    if editor.is_probing() {
        editor.mark_probed_change(would_change);
        return;
    }
    if would_change {
        apply(editor);
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::TrackedRanges],
        });
        editor.push_event(EditorEvent::RenderInvalidated);
    }
}

fn commit_or_probe<F>(editor: &mut Editor, would_change: bool, apply: F)
where
    F: FnOnce(&mut crate::tracked_range::TrackedRangeRegistry),
{
    if editor.is_probing() {
        editor.mark_probed_change(would_change);
        return;
    }
    if would_change {
        apply(editor.tracked_ranges_mut());
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::TrackedRanges],
        });
        editor.push_event(EditorEvent::RenderInvalidated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    use crate::test_utils::assert_probe_predicts_apply;

    fn add_frozen_op(id: &str, stable: editor_state::StableSelection) -> Message {
        Message::TrackedRange {
            op: TrackedRangeOp::AddFrozen {
                id: id.into(),
                group: "g1".into(),
                selection: stable,
                metadata: String::new(),
            },
        }
    }

    fn add_op(id: &str, sel: editor_state::Selection) -> Message {
        Message::TrackedRange {
            op: TrackedRangeOp::Add {
                id: id.into(),
                group: "g1".into(),
                selection: sel,
                metadata: String::new(),
            },
        }
    }

    #[test]
    fn add_inserts_range_and_emits_state_changed() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        let events = editor.apply(add_op("a", sel));
        assert!(editor.tracked_ranges().contains("a"));
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn add_same_range_twice_is_idempotent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        editor.apply(add_op("a", sel));
        let events = editor.apply(add_op("a", sel));
        assert!(!events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn remove_drops_range_and_emits_state_changed() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        editor.apply(add_op("a", sel));
        let events = editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Remove { id: "a".into() },
        });
        assert!(!editor.tracked_ranges().contains("a"));
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn remove_nonexistent_does_not_emit_event() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        let events = editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Remove { id: "x".into() },
        });
        assert!(!events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn invalidate_sets_flag() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        editor.apply(add_op("a", sel));
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Invalidate { id: "a".into() },
        });
        assert!(editor.tracked_ranges().get("a").unwrap().explicitly_invalid);
    }

    #[test]
    fn clear_group_empties_group() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        editor.apply(add_op("a", sel));
        editor.apply(add_op("b", sel));
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::ClearGroup { group: "g1".into() },
        });
        assert_eq!(editor.tracked_ranges().len(), 0);
    }

    #[test]
    fn probe_add_predicts_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        assert_probe_predicts_apply(state, add_op("a", sel));
    }

    #[test]
    fn probe_remove_nonexistent_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_probe_predicts_apply(
            state,
            Message::TrackedRange {
                op: TrackedRangeOp::Remove { id: "x".into() },
            },
        );
    }

    #[test]
    fn probe_clear_empty_group_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_probe_predicts_apply(
            state,
            Message::TrackedRange {
                op: TrackedRangeOp::ClearGroup {
                    group: "nothing".into(),
                },
            },
        );
    }

    #[test]
    fn probe_invalidate_already_invalid_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let mut editor = Editor::new_test(state);
        editor.apply(add_op("a", sel));
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Invalidate { id: "a".into() },
        });
        let probed = editor
            .can(Message::TrackedRange {
                op: TrackedRangeOp::Invalidate { id: "a".into() },
            })
            .unwrap();
        assert!(!probed);
    }

    #[test]
    fn add_frozen_inserts_range_and_emits_events() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = editor_state::StableSelection::capture(&sel, &state.view());
        let mut editor = Editor::new_test(state);
        let events = editor.apply(add_frozen_op("a", stable));
        assert!(editor.tracked_ranges().contains("a"));
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated)),
            "AddFrozen must also emit RenderInvalidated (spec §5.3)"
        );
    }

    #[test]
    fn add_frozen_same_range_twice_is_idempotent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = editor_state::StableSelection::capture(&sel, &state.view());
        let mut editor = Editor::new_test(state);
        editor.apply(add_frozen_op("a", stable.clone()));
        let events = editor.apply(add_frozen_op("a", stable));
        assert!(!events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn add_frozen_yields_same_registry_as_add() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = editor_state::StableSelection::capture(&sel, &state.view());

        let mut a = Editor::new_test(state.clone());
        a.apply(add_op("r", sel));

        let mut b = Editor::new_test(state);
        b.apply(add_frozen_op("r", stable));

        assert_eq!(a.tracked_ranges().get("r"), b.tracked_ranges().get("r"));
    }

    #[test]
    fn probe_add_frozen_predicts_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = editor_state::StableSelection::capture(&sel, &state.view());
        assert_probe_predicts_apply(state, add_frozen_op("a", stable));
    }

    #[test]
    fn probe_add_frozen_same_id_same_content_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = editor_state::StableSelection::capture(&sel, &state.view());

        let mut editor = Editor::new_test(state);
        editor.apply(add_frozen_op("a", stable.clone()));

        let probed = editor.can(add_frozen_op("a", stable)).unwrap();
        assert!(
            !probed,
            "AddFrozen with same id + same content must predict no change"
        );
    }

    fn sample_style() -> DecorationStyle {
        DecorationStyle {
            background: Some("selection".into()),
            underline: None,
            ..Default::default()
        }
    }

    fn set_group_op(group: &str, style: DecorationStyle, enabled: bool) -> Message {
        Message::TrackedRange {
            op: TrackedRangeOp::SetGroupDecoration {
                group: group.into(),
                style,
                enabled,
                z_index: 0,
            },
        }
    }

    #[test]
    fn set_group_decoration_stores_in_view_state() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let events = editor.apply(set_group_op("g1", sample_style(), true));
        let stored = editor.view().view_state().group_decoration("g1").unwrap();
        assert_eq!(stored.style, sample_style());
        assert!(stored.enabled);
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn set_group_decoration_same_value_is_idempotent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(set_group_op("g1", sample_style(), true));
        let events = editor.apply(set_group_op("g1", sample_style(), true));
        assert!(!events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn set_group_decoration_toggle_enabled_emits_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(set_group_op("g1", sample_style(), true));
        let events = editor.apply(set_group_op("g1", sample_style(), false));
        let stored = editor.view().view_state().group_decoration("g1").unwrap();
        assert!(!stored.enabled);
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn remove_group_decoration_drops_entry() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(set_group_op("g1", sample_style(), true));
        let events = editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::RemoveGroupDecoration { group: "g1".into() },
        });
        assert!(editor.view().view_state().group_decoration("g1").is_none());
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn remove_unknown_group_decoration_is_noop() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let events = editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::RemoveGroupDecoration {
                group: "missing".into(),
            },
        });
        assert!(!events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::TrackedRanges)
        )));
    }

    #[test]
    fn probe_set_group_decoration_predicts_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(state, set_group_op("g1", sample_style(), true));
    }

    #[test]
    fn probe_remove_unknown_group_decoration_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(
            state,
            Message::TrackedRange {
                op: TrackedRangeOp::RemoveGroupDecoration {
                    group: "missing".into(),
                },
            },
        );
    }
}
