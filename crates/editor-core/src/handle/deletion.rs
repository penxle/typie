use std::sync::Arc;

use editor_commands::{self as commands, CommandError, CommandResult};
use editor_state::{
    Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, flat_size, flat_text,
    remap_selection,
};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_deletion_op(editor: &mut Editor, op: DeletionOp) -> Result<(), EditorError> {
    match op {
        DeletionOp::Selection => {
            editor.transact(|tr| {
                commands::delete_selection(tr)?;
                Ok(())
            })?;
        }
        DeletionOp::Move {
            movement:
                Movement::Grapheme {
                    direction: Direction::Backward,
                },
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::delete_text_backward(tr, &resource)?;
                Ok(())
            })?;
        }
        DeletionOp::Move {
            movement:
                Movement::Grapheme {
                    direction: Direction::Forward,
                },
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::delete_text_forward(tr, &resource)?;
                Ok(())
            })?;
        }
        DeletionOp::Surrounding { before, after } => {
            editor.transact(|tr| {
                let (before_chars, after_chars) = utf16_to_char_counts(tr, before, after)?;
                delete_surrounding(tr, before_chars, after_chars)?;
                Ok(())
            })?;
        }
        DeletionOp::SurroundingCodePoints { before, after } => {
            editor.transact(|tr| {
                delete_surrounding(tr, before, after)?;
                Ok(())
            })?;
        }
        DeletionOp::Move { movement } => {
            let Some(input_state) = editor.layout_input_state() else {
                return Ok(());
            };
            let Some(selection) = input_state.selection else {
                return Ok(());
            };
            let head = selection.head;
            let resource_guard = editor.resource.lock().unwrap();
            let target = editor
                .view
                .resolve_movement(&head, &movement, &resource_guard);
            drop(resource_guard);

            if let Some(target) = target {
                let selection = Selection::new(head, target.head);
                let Some(selection) = remap_selection(selection, &input_state, &editor.state)
                else {
                    return Ok(());
                };
                editor.transact(|tr| {
                    commands::set_selection(tr, selection)?;
                    commands::delete_selection(tr)?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

fn replace_text_range(tr: &mut Transaction, start: usize, end: usize, text: &str) -> CommandResult {
    let (start_pos, end_pos): (Position, Position) = {
        let doc = tr.view();
        let start_pos = (&ResolvedPosition::from_flat(&doc, start)
            .ok_or(CommandError::Corrupted("flat start unresolvable".into()))?)
            .into();
        let end_pos = (&ResolvedPosition::from_flat(&doc, end)
            .ok_or(CommandError::Corrupted("flat end unresolvable".into()))?)
            .into();
        (start_pos, end_pos)
    };

    commands::chain!(
        tr,
        commands::set_selection(Selection::new(start_pos, end_pos)),
        commands::when!(start != end, commands::delete_selection()),
        commands::when!(!text.is_empty(), commands::insert_text(text)),
    )
}

fn delete_surrounding(tr: &mut Transaction, before: usize, after: usize) -> CommandResult {
    let doc = tr.view();
    let selection = tr
        .selection()
        .ok_or(CommandError::Corrupted("no selection".into()))?;
    let cursor_flat = selection
        .head
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cursor unresolvable".into()))?
        .to_flat();

    let (before_anchor, after_anchor) = match tr.composition() {
        Some(comp) => (comp.start.min(cursor_flat), comp.end.max(cursor_flat)),
        None => (cursor_flat, cursor_flat),
    };

    let doc_size = flat_size(&doc);
    let del_start = before_anchor.saturating_sub(before);
    let del_end_after = after_anchor.saturating_add(after).min(doc_size);

    let before_count = before_anchor - del_start;
    let any_delete = del_end_after > after_anchor || before_anchor > del_start;

    if del_end_after > after_anchor {
        replace_text_range(tr, after_anchor, del_end_after, "")?;
    }
    if before_anchor > del_start {
        replace_text_range(tr, del_start, before_anchor, "")?;
    }

    // replace_text_range leaves cursor at del_start, which is correct when there is no
    // composition (before_anchor == cursor_flat). With composition the cursor must shift
    // left by before_count to preserve its logical position relative to the surrounding text.
    if any_delete {
        let new_cursor_flat = cursor_flat - before_count;
        let doc_after = tr.view();
        let resolved = ResolvedPosition::from_flat(&doc_after, new_cursor_flat)
            .ok_or(CommandError::Corrupted("cursor restore failed".into()))?;
        commands::set_selection(tr, Selection::collapsed((&resolved).into()))?;
    }

    Ok(true)
}

fn utf16_to_char_counts(
    tr: &Transaction,
    before_u16: usize,
    after_u16: usize,
) -> Result<(usize, usize), CommandError> {
    let doc = tr.view();
    let selection = tr
        .selection()
        .ok_or(CommandError::Corrupted("no selection".into()))?;
    let cursor_flat = selection
        .head
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cursor unresolvable".into()))?
        .to_flat();
    let doc_size = flat_size(&doc);

    let before_window_start = cursor_flat.saturating_sub(before_u16);
    let after_window_end = cursor_flat.saturating_add(after_u16).min(doc_size);
    let text_before = flat_text(&doc, before_window_start..cursor_flat);
    let text_after = flat_text(&doc, cursor_flat..after_window_end);

    Ok((
        utf16_count_backward_as_chars(&text_before, before_u16),
        utf16_count_forward_as_chars(&text_after, after_u16),
    ))
}

fn utf16_count_backward_as_chars(text: &str, target: usize) -> usize {
    let mut chars = 0;
    let mut units = 0;
    for c in text.chars().rev() {
        let c_units = c.len_utf16();
        if units + c_units > target {
            break;
        }
        units += c_units;
        chars += 1;
    }
    chars
}

fn utf16_count_forward_as_chars(text: &str, target: usize) -> usize {
    let mut chars = 0;
    let mut units = 0;
    for c in text.chars() {
        let c_units = c.len_utf16();
        if units + c_units > target {
            break;
        }
        units += c_units;
        chars += 1;
    }
    chars
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::Resource;
    use editor_state::assert_state_eq;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::editor::Editor;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_delete_selection_collapsed_at_start() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(
            state,
            Message::Deletion {
                op: DeletionOp::Selection,
            },
        );
    }

    #[test]
    fn probe_delete_selection_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_probe_predicts_apply(
            state,
            Message::Deletion {
                op: DeletionOp::Selection,
            },
        );
    }

    #[test]
    fn delete_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2) -> (p1, 8)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("herld") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_forward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    fn editor_with_layout(state: editor_state::State) -> Editor {
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let mut editor = Editor::new_test_with_resource(state.clone(), resource);
        editor.view.layout(&state);
        editor
    }

    #[test]
    fn delete_word_backward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 11)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Word {
                    direction: Direction::Backward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello ") } } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_forward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Word {
                    direction: Direction::Forward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text(" world") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_backward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Line {
                    direction: Direction::Backward,
                    axis: editor_common::Axis::Horizontal,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text(" world") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_forward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Line {
                    direction: Direction::Forward,
                    axis: editor_common::Axis::Horizontal,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_at_doc_start_is_noop() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Word {
                    direction: Direction::Backward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward_combining_mark() {
        // "aéb" = a + e + U+0301 + b = 4 codepoints, 3 graphemes
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ae\u{0301}b") } } }
            selection: (p1, 3)
        };
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let mut editor = Editor::new_test_with_resource(state, resource);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward_zwj_emoji_sequence() {
        // "a😶‍🌫️b" = a + (U+1F636 ZWJ U+1F32B FE0F) + b = 6 codepoints, 3 graphemes
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a\u{1F636}\u{200D}\u{1F32B}\u{FE0F}b") } } }
            selection: (p1, 5)
        };
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let mut editor = Editor::new_test_with_resource(state, resource);
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
