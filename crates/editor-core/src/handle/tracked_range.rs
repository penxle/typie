use editor_state::StableSelection;
use editor_view::GroupDecoration;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
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
            let doc = &editor.state().doc;
            if selection.anchor.resolve(doc).is_none() || selection.head.resolve(doc).is_none() {
                return Err(EditorError::General {
                    msg: "TrackedRange::Add: selection must resolve against current doc".into(),
                });
            }
            let new_range = TrackedRange {
                id: id.clone(),
                group,
                selection: StableSelection::freeze(&selection, doc),
                metadata,
                explicitly_invalid: false,
            };
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
        } => {
            let decoration = GroupDecoration { style, enabled };
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
    }
    Ok(())
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let sel = state.selection.unwrap();
        assert_probe_predicts_apply(state, add_op("a", sel));
    }

    #[test]
    fn probe_remove_nonexistent_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
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

    fn sample_style() -> DecorationStyle {
        DecorationStyle {
            background: Some("selection".into()),
            underline: None,
        }
    }

    fn set_group_op(group: &str, style: DecorationStyle, enabled: bool) -> Message {
        Message::TrackedRange {
            op: TrackedRangeOp::SetGroupDecoration {
                group: group.into(),
                style,
                enabled,
            },
        }
    }

    #[test]
    fn set_group_decoration_stores_in_view_state() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        assert_probe_predicts_apply(state, set_group_op("g1", sample_style(), true));
    }

    #[test]
    fn probe_remove_unknown_group_decoration_predicts_no_change() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
