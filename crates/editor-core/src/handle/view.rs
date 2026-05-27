use editor_model::{Doc, Node, NodeId};
use editor_state::{Position, Selection};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::*;

pub fn handle_view_op(editor: &mut Editor, op: ViewOp) -> Result<(), EditorError> {
    match op {
        ViewOp::ToggleFold { id } => {
            let was_expanded = editor.fold_expanded(id);
            if was_expanded
                && let Some(sel) = editor.state.selection
                && let Some(remapped) =
                    remap_selection_out_of_fold_content(&editor.state.doc, id, sel)
            {
                // fold toggle is non-undoable view state; the coupled remap must
                // skip history too, else undo strands the caret in still-collapsed
                // content (cf. handle/selection.rs).
                editor.transact(|tr| {
                    tr.update_meta(|m| m.history = HistoryMeta::Skip);
                    tr.set_selection(Some(remapped))?;
                    Ok(())
                })?;
            }
            editor.toggle_fold(id);
            Ok(())
        }
        ViewOp::ScrollIntoView { target } => handle_scroll_into_view(editor, target),
    }
}

fn handle_scroll_into_view(editor: &mut Editor, target: ScrollTarget) -> Result<(), EditorError> {
    let selection = match target {
        ScrollTarget::TrackedItem { id } => {
            let Some(range) = editor.tracked_ranges().get(&id).cloned() else {
                return Ok(());
            };
            if range.explicitly_invalid {
                return Ok(());
            }
            let sel = range.selection.thaw(&editor.state.doc);
            if sel.is_collapsed() {
                return Ok(());
            }
            sel
        }
        ScrollTarget::Selection => {
            let Some(sel) = editor.state.selection else {
                return Ok(());
            };
            if sel.is_collapsed() {
                return Ok(());
            }
            sel
        }
    };

    let endpoints = selection
        .resolve(&editor.state.doc)
        .and_then(|resolved| editor.view.selection_endpoints(&resolved));
    let Some(endpoints) = endpoints else {
        return Ok(());
    };

    if editor.is_probing() {
        editor.mark_probed_change(true);
        return Ok(());
    }

    editor.push_event(EditorEvent::Scroll {
        rect: endpoints.from,
    });
    Ok(())
}

// Legacy parity: collapse hides fold-content, so a caret/anchor inside it is
// moved to the fold-title end or it would be stranded in invisible content.
// Mode-agnostic.
fn remap_selection_out_of_fold_content(
    doc: &Doc,
    fold_id: NodeId,
    sel: Selection,
) -> Option<Selection> {
    let fold = doc.node(fold_id)?;
    if !matches!(fold.node(), Node::Fold(_)) {
        return None;
    }
    let mut fold_title_id = None;
    let mut fold_content_id = None;
    for child in fold.children() {
        match child.node() {
            Node::FoldTitle(_) => fold_title_id = Some(child.id()),
            Node::FoldContent(_) => fold_content_id = Some(child.id()),
            _ => {}
        }
    }
    let fold_title_id = fold_title_id?;
    let fold_content_id = fold_content_id?;

    let in_content = |nid: NodeId| {
        nid == fold_content_id
            || doc
                .node(nid)
                .is_some_and(|n| n.ancestors().any(|a| a.id() == fold_content_id))
    };
    let anchor_in = in_content(sel.anchor.node_id);
    let head_in = in_content(sel.head.node_id);
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

    use super::*;
    use crate::event::EditorEvent;
    use crate::state_field::StateField;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_toggle_fold_existing() {
        let (state, f1, ..) = state! {
            doc { root {
                f1: fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (t1, 0)
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
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (t1, 0)
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
        let (initial, f1, t2) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        editor.apply(Message::View {
            op: ViewOp::ToggleFold { id: f1 },
        });

        assert_ne!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .head
                .node_id,
            t2,
            "selection inside fold-content must be remapped out on collapse"
        );
    }

    #[test]
    fn collapse_then_undo_keeps_selection_out_of_fold_content() {
        let (initial, f1, t2) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
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
                .node_id,
            t2,
            "undo must not restore a selection inside collapsed fold-content"
        );
    }

    #[test]
    fn collapse_keeps_selection_outside_fold() {
        let (initial, f1, t1) = state! {
            doc { root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
                paragraph { t1: text("Out") }
            } }
            selection: (t1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
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
                .node_id,
            t1,
            "selection outside the fold is untouched"
        );
    }
}
