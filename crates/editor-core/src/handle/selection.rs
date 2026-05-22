use editor_commands::{self as commands};
use editor_state::{
    Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, farther_endpoint,
};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_selection_op(editor: &mut Editor, op: SelectionOp) -> Result<(), EditorError> {
    let extend_to_selection = resolve_extend_to_selection(editor, &op);

    editor.view.clear_preferred_x();
    editor.transact(|tr| {
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        match op {
            SelectionOp::Set { selection } => {
                commands::set_selection(tr, selection)?;
            }
            SelectionOp::All => {
                commands::select_all(tr)?;
            }
            SelectionOp::SetFlat { start, end } => {
                let doc = tr.doc();
                let start_pos = match ResolvedPosition::from_flat(&doc, start) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let end_pos = match ResolvedPosition::from_flat(&doc, end) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                commands::set_selection(
                    tr,
                    Selection::new(Position::from(&start_pos), Position::from(&end_pos)),
                )?;
            }
            SelectionOp::ExtendTo { .. } => {
                if let Some(selection) = extend_to_selection
                    && tr.selection() != selection
                {
                    commands::set_selection(tr, selection)?;
                }
            }
        }
        Ok(())
    })
}

fn resolve_extend_to_selection(editor: &Editor, op: &SelectionOp) -> Option<Selection> {
    let SelectionOp::ExtendTo {
        anchor_page,
        anchor_x,
        anchor_y,
        head_page,
        head_x,
        head_y,
        initial_selection,
    } = op
    else {
        return None;
    };

    let doc = &editor.state().doc;
    let head_hit = editor
        .view
        .hit_test_extending(*head_page, *head_x, *head_y)?;

    if let Some(initial_selection) = initial_selection {
        let initial = initial_selection.resolve(doc)?;
        let head = head_hit.resolve(doc)?;
        let initial_from = Position::from(initial.from());
        let initial_to = Position::from(initial.to());

        return Some(if head.from() < initial.from() {
            let head = farther_endpoint(doc, initial_to, head_hit.anchor, head_hit.head);
            Selection::new(initial_to, head)
        } else if head.to() > initial.to() {
            let head = farther_endpoint(doc, initial_from, head_hit.anchor, head_hit.head);
            Selection::new(initial_from, head)
        } else {
            Selection::new(initial_from, initial_to)
        });
    }

    editor
        .view
        .hit_test_extending(*anchor_page, *anchor_x, *anchor_y)
        .map(|anchor_hit| {
            let anchor = farther_endpoint(doc, head_hit.head, anchor_hit.anchor, anchor_hit.head);
            let head = farther_endpoint(doc, anchor, head_hit.anchor, head_hit.head);
            Selection::new(anchor, head)
        })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Position, Selection};

    use super::*;

    #[test]
    fn select_set() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let target = Selection::collapsed(Position::new(t1, 3));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });
        assert_eq!(editor.state().selection, target);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_with_initial_selection_expands_word_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let initial = editor.state().selection;
        assert_ne!(initial.anchor, initial.head);

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor_page: 0,
                anchor_x: 0.0,
                anchor_y: 5.0,
                head_page: 0,
                head_x: 9999.0,
                head_y: 5.0,
                initial_selection: Some(initial),
            },
        });

        let sel = editor.state().selection;
        assert_eq!(sel.anchor, initial.anchor);
        assert_eq!(sel.head.node_id, initial.head.node_id);
        assert!(
            sel.head.offset > initial.head.offset,
            "selection should extend beyond the initially selected word, got {:?}",
            sel
        );
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_without_initial_selection_allows_collapsed_promoted_slot() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor_page: 0,
                anchor_x: 20.0,
                anchor_y: -100.0,
                head_page: 0,
                head_x: 20.0,
                head_y: -100.0,
                initial_selection: None,
            },
        });

        assert_ne!(editor.state().selection, before);
        assert!(editor.state().selection.is_collapsed());
        assert!(!editor.history.can_undo());
    }
}
