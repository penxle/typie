use editor_crdt::Dot;
use editor_model::{DocView, Node};
use editor_state::{Position, Selection};
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
    }
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
    let mut fold_title_id = None;
    let mut fold_content_id = None;
    for child in fold.child_blocks() {
        match child.node() {
            Node::FoldTitle(_) => fold_title_id = Some(child.id()),
            Node::FoldContent(_) => fold_content_id = Some(child.id()),
            _ => {}
        }
    }
    let fold_title_id = fold_title_id?;
    let fold_content_id = fold_content_id?;

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
    use crate::test_utils::assert_probe_predicts_apply;

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
    fn probe_toggle_fold_existing() {
        let (state, f1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        assert_probe_predicts_apply(
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
