use editor_crdt::Dot;
use editor_model::{DocView, Node};
use editor_state::{Position, Selection, StableResolveCtx};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_view_op(editor: &mut Editor, op: ViewOp) -> Result<(), EditorError> {
    match op {
        ViewOp::ToggleFold { id } => {
            let was_expanded = editor.fold_expanded(id);
            if was_expanded
                && let Some(sel) = editor.state.selection
                && let Some(remapped) =
                    remap_selection_out_of_fold_content(&editor.state.view(), id, sel)
            {
                // fold toggle is non-undoable view state; the coupled remap must
                // skip history too, else undo strands the caret in still-collapsed
                // content (cf. handle/selection.rs).
                editor.transact(|tr| {
                    tr.update_meta(|m| m.history = HistoryMeta::Skip);
                    if tr.selection() != Some(remapped) {
                        tr.clear_pending_format()?;
                    }
                    tr.set_selection(Some(remapped))?;
                    Ok(())
                })?;
            }
            editor.toggle_fold(id);
            Ok(())
        }
        ViewOp::ExpandFoldsForTrackedRange { id } => {
            let folds = folds_containing_tracked_range(editor, &id);
            editor.expand_folds(folds);
            Ok(())
        }
    }
}

fn folds_containing_tracked_range(editor: &Editor, id: &str) -> Vec<Dot> {
    let Some(range) = editor.tracked_ranges().get(id) else {
        return Vec::new();
    };
    if range.locate(editor.state()).is_none() {
        return Vec::new();
    }
    let doc = editor.state().view();
    let ctx = StableResolveCtx::from_live(&doc, editor.state().projected.seq_checkout());
    let Some(selection) = range.selection.resolve(&ctx) else {
        return Vec::new();
    };
    let mut folds = Vec::new();
    for position in [selection.anchor, selection.head] {
        let Some(node) = doc.node(position.node) else {
            continue;
        };
        for ancestor in node.ancestors() {
            if !matches!(ancestor.node(), Node::FoldContent(_)) {
                continue;
            }
            let Some(fold) = ancestor.parent() else {
                continue;
            };
            if matches!(fold.node(), Node::Fold(_)) && !folds.contains(&fold.id()) {
                folds.push(fold.id());
            }
        }
    }
    folds
}

// Legacy parity: collapse hides fold-content, so a caret/anchor inside it is
// moved to the fold-title end or it would be stranded in invisible content.
// Mode-agnostic.
fn remap_selection_out_of_fold_content(
    doc: &DocView,
    fold_id: Dot,
    sel: Selection,
) -> Option<Selection> {
    let fold = doc.node(fold_id)?;
    if !matches!(fold.node(), Node::Fold(_)) {
        return None;
    }
    let fold_title_id = fold.fold_title()?.id();
    let fold_content_id = fold.fold_content()?.id();

    let in_content = |nid: Dot| {
        nid == fold_content_id
            || doc
                .node(nid)
                .is_some_and(|n| n.ancestors().any(|a| a.id() == fold_content_id))
    };
    let anchor_in = in_content(sel.anchor.node);
    let head_in = in_content(sel.head.node);
    if !anchor_in && !head_in {
        return None;
    }

    let title_children = doc.node(fold_title_id)?.children().count();
    let target = Position::new(fold_title_id, title_children);
    let anchor = if anchor_in { target } else { sel.anchor };
    let head = if head_in { target } else { sel.head };
    Some(Selection::new(anchor, head))
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;
    use editor_state::PendingModifier;

    use super::*;
    use crate::event::EditorEvent;
    use crate::state_field::StateField;
    use crate::test_utils::assert_apply_changes_state;

    fn set_pending_format(editor: &mut Editor) {
        editor
            .transact(|tr| {
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();
    }

    fn assert_pending_format_cleared(editor: &Editor) {
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending modifiers cleared"
        );
    }

    fn add_tracked_range(editor: &mut Editor, id: &str, selection: Selection) {
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Add {
                id: id.into(),
                group: "test".into(),
                selection,
                metadata: String::new(),
                invalidate_on_text_change: false,
            },
        });
    }

    #[test]
    fn fold_defaults_to_collapsed_on_load() {
        let (initial, f1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        assert!(!editor.fold_expanded(f1), "folds load collapsed by default");
    }

    #[test]
    fn expand_folds_for_tracked_range_opens_containing_fold_content() {
        let (initial, outer, _outer_title, inner, p1) = state! {
            doc { root {
                outer: fold {
                    outer_title: fold_title { text("Outer") }
                    fold_content {
                        inner: fold {
                            fold_title { text("Inner") }
                            fold_content { p1: paragraph { text("Body") } }
                        }
                    }
                }
            } }
            selection: (outer_title, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        add_tracked_range(
            &mut editor,
            "range",
            Selection::new(Position::new(p1, 0), Position::new(p1, 4)),
        );
        set_pending_format(&mut editor);
        let selection_before = editor.state().selection;
        let history_undos_before = editor.history_undos_len();
        let history_redos_before = editor.history_redos_len();

        let events = editor.apply(Message::View {
            op: ViewOp::ExpandFoldsForTrackedRange { id: "range".into() },
        });

        assert!(editor.fold_expanded(outer));
        assert!(editor.fold_expanded(inner));
        assert_eq!(editor.state().selection, selection_before);
        assert!(!editor.state().pending_modifiers.is_empty());
        assert_eq!(editor.history_undos_len(), history_undos_before);
        assert_eq!(editor.history_redos_len(), history_redos_before);
        let expected_fields = [
            StateField::Cursor,
            StateField::PageSizes,
            StateField::ExternalElements,
            StateField::TableOverlays,
            StateField::LinkRects,
            StateField::TrackedRanges,
        ];
        assert!(events.iter().any(|event| matches!(
            event,
            EditorEvent::StateChanged { fields }
                if expected_fields.iter().all(|field| fields.contains(field))
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, EditorEvent::RenderInvalidated))
        );

        let repeated_events = editor.apply(Message::View {
            op: ViewOp::ExpandFoldsForTrackedRange { id: "range".into() },
        });
        assert!(repeated_events.is_empty());
        assert_eq!(editor.history_undos_len(), history_undos_before);
        assert_eq!(editor.history_redos_len(), history_redos_before);
    }

    #[test]
    fn expand_folds_for_tracked_range_opens_sibling_endpoint_folds_only() {
        let (initial, first, _first_title, p1, second, p2, unrelated) = state! {
            doc { root {
                first: fold {
                    first_title: fold_title { text("First") }
                    fold_content { p1: paragraph { text("One") } }
                }
                second: fold {
                    fold_title { text("Second") }
                    fold_content { p2: paragraph { text("Two") } }
                }
                unrelated: fold {
                    fold_title { text("Unrelated") }
                    fold_content { paragraph { text("Other") } }
                }
            } }
            selection: (first_title, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        add_tracked_range(
            &mut editor,
            "spanning-range",
            Selection::new(Position::new(p1, 0), Position::new(p2, 3)),
        );

        editor.apply(Message::View {
            op: ViewOp::ExpandFoldsForTrackedRange {
                id: "spanning-range".into(),
            },
        });

        assert!(editor.fold_expanded(first));
        assert!(editor.fold_expanded(second));
        assert!(!editor.fold_expanded(unrelated));
    }

    #[test]
    fn expand_folds_for_tracked_range_does_not_open_fold_for_title() {
        let (initial, f1, ft1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        add_tracked_range(
            &mut editor,
            "title-range",
            Selection::new(Position::new(ft1, 0), Position::new(ft1, 5)),
        );

        let events = editor.apply(Message::View {
            op: ViewOp::ExpandFoldsForTrackedRange {
                id: "title-range".into(),
            },
        });

        assert!(!editor.fold_expanded(f1));
        assert!(events.is_empty());
    }

    #[test]
    fn expand_folds_for_missing_tracked_range_is_noop() {
        let (initial, f1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let events = editor.apply(Message::View {
            op: ViewOp::ExpandFoldsForTrackedRange {
                id: "missing".into(),
            },
        });

        assert!(!editor.fold_expanded(f1));
        assert!(events.is_empty());
    }

    #[test]
    fn toggle_fold_existing_changes_state() {
        let (state, f1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        assert_apply_changes_state(
            state,
            Message::View {
                op: ViewOp::ToggleFold { id: f1 },
            },
        );
    }

    #[test]
    fn toggle_fold_relayouts_and_emits_events() {
        let (initial, f1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let events = editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::PageSizes)
        )));
    }

    #[test]
    fn collapse_remaps_selection_out_of_fold_content() {
        let (initial, f1, p1) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { p1: paragraph { text("Body") } }
                }
            } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });
        assert!(editor.fold_expanded(f1), "first toggle expands the fold");
        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });

        assert_ne!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .head
                .node,
            p1,
            "selection inside fold-content must be remapped out on collapse"
        );
    }

    #[test]
    fn collapse_remap_clears_pending_format() {
        let (initial, f1, _p1) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { p1: paragraph { text("Body") } }
                }
            } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });
        set_pending_format(&mut editor);

        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });

        assert_pending_format_cleared(&editor);
    }

    #[test]
    fn collapse_then_undo_keeps_selection_out_of_fold_content() {
        let (initial, f1, p1) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { p1: paragraph { text("Body") } }
                }
            } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });
        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_ne!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .head
                .node,
            p1,
            "undo must not restore a selection inside collapsed fold-content"
        );
    }

    #[test]
    fn collapse_keeps_selection_outside_fold() {
        let (initial, f1, p1) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
                p1: paragraph { text("Out") }
            } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });
        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });

        assert_eq!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .head
                .node,
            p1,
            "selection outside the fold is untouched"
        );
    }
}
