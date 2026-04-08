use std::sync::Arc;

use editor_commands::{self as commands, CommandError, CommandResult};
use editor_schema::{DocFlatExt, ResolvedPositionFlatExt};
use editor_state::{ResolvedPosition, Selection};
use editor_transaction::Transaction;

use super::helpers::replace_flat_range;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_deletion_intent(
    editor: &mut Editor,
    intent: DeletionIntent,
) -> Result<(), EditorError> {
    match intent {
        DeletionIntent::Selection => {
            editor.transact(|tr| {
                commands::delete_selection(tr)?;
                Ok(())
            })?;
        }
        DeletionIntent::Move {
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
        DeletionIntent::Move {
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
        DeletionIntent::Move { movement } => {
            let head = editor.state().selection.head;
            let resource_guard = editor.resource.lock().unwrap();
            let target = editor
                .view
                .resolve_movement(&head, &movement, &resource_guard);
            drop(resource_guard);

            if let Some(target) = target {
                let selection = Selection::new(head, target.head);
                editor.transact(|tr| {
                    commands::set_selection(tr, selection)?;
                    commands::delete_selection(tr)?;
                    Ok(())
                })?;
            }
        }
        DeletionIntent::Surrounding { before, after } => {
            editor.transact(|tr| {
                let (before_chars, after_chars) = utf16_to_char_counts(tr, before, after)?;
                delete_surrounding(tr, before_chars, after_chars)?;
                Ok(())
            })?;
        }
        DeletionIntent::SurroundingCodePoints { before, after } => {
            editor.transact(|tr| {
                delete_surrounding(tr, before, after)?;
                Ok(())
            })?;
        }
    }
    Ok(())
}

fn delete_surrounding(tr: &mut Transaction, before: usize, after: usize) -> CommandResult {
    let doc = tr.doc();
    let cursor_flat = tr
        .selection()
        .head
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cursor unresolvable".into()))?
        .to_flat();

    let (before_anchor, after_anchor) = match tr.composition() {
        Some(comp) => (comp.start.min(cursor_flat), comp.end.max(cursor_flat)),
        None => (cursor_flat, cursor_flat),
    };

    let doc_size = doc.flat_size();
    let del_start = before_anchor.saturating_sub(before);
    let del_end_after = after_anchor.saturating_add(after).min(doc_size);

    let before_count = before_anchor - del_start;
    let any_delete = del_end_after > after_anchor || before_anchor > del_start;

    if del_end_after > after_anchor {
        replace_flat_range(tr, after_anchor, del_end_after, "")?;
    }
    if before_anchor > del_start {
        replace_flat_range(tr, del_start, before_anchor, "")?;
    }

    // replace_flat_range leaves cursor at del_start, which is correct when there is no
    // composition (before_anchor == cursor_flat). With composition the cursor must shift
    // left by before_count to preserve its logical position relative to the surrounding text.
    if any_delete {
        let new_cursor_flat = cursor_flat - before_count;
        let doc_after = tr.doc();
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
    let doc = tr.doc();
    let cursor_flat = tr
        .selection()
        .head
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cursor unresolvable".into()))?
        .to_flat();
    let doc_size = doc.flat_size();

    let before_window_start = cursor_flat.saturating_sub(before_u16);
    let after_window_end = cursor_flat.saturating_add(after_u16).min(doc_size);
    let text_before = doc.flat_text(before_window_start..cursor_flat);
    let text_after = doc.flat_text(cursor_flat..after_window_end);

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
    use editor_resource::TextSegmenters;
    use editor_state::assert_state_eq;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::editor::Editor;

    #[test]
    fn delete_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 2) -> (t1, 8)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Selection,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("herld") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    fn editor_with_layout(state: editor_state::State) -> Editor {
        let resource = Arc::new(Mutex::new(Resource::new()));
        resource.lock().unwrap().segmenters = Some(Arc::new(TextSegmenters::new_test()));
        let mut editor = Editor::new_test_with_resource(state.clone(), resource);
        editor.view.layout(&state.doc);
        editor
    }

    #[test]
    fn delete_word_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello ") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Forward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text(" world") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Line {
                        direction: Direction::Backward,
                        axis: editor_common::Axis::Horizontal,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text(" world") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Line {
                        direction: Direction::Forward,
                        axis: editor_common::Axis::Horizontal,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_at_doc_start_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_without_segmenters_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn utf16_backward_bmp_only() {
        assert_eq!(super::utf16_count_backward_as_chars("hello", 3), 3);
        assert_eq!(super::utf16_count_backward_as_chars("hello", 0), 0);
        assert_eq!(super::utf16_count_backward_as_chars("hello", 10), 5);
    }

    #[test]
    fn utf16_backward_surrogate_pair() {
        // emoji 😀 = 2 UTF-16 units = 1 char
        let text = "ab😀cd"; // a, b, 😀, c, d (5 chars, 6 UTF-16 units)
        // counting 2 UTF-16 units backward from end: "cd" = 2 chars
        assert_eq!(super::utf16_count_backward_as_chars(text, 2), 2);
        // 3 UTF-16 units backward = "cd" + can't split 😀 → round-down to 2 chars
        assert_eq!(super::utf16_count_backward_as_chars(text, 3), 2);
        // 4 UTF-16 units = 😀cd = 3 chars
        assert_eq!(super::utf16_count_backward_as_chars(text, 4), 3);
    }

    #[test]
    fn utf16_forward_bmp_only() {
        assert_eq!(super::utf16_count_forward_as_chars("hello", 3), 3);
    }

    #[test]
    fn utf16_forward_surrogate_pair() {
        let text = "😀ab"; // 😀, a, b
        assert_eq!(super::utf16_count_forward_as_chars(text, 1), 0); // can't split
        assert_eq!(super::utf16_count_forward_as_chars(text, 2), 1); // 😀
        assert_eq!(super::utf16_count_forward_as_chars(text, 3), 2); // 😀a
    }

    #[test]
    fn surrounding_code_points_deletes_around_cursor() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 2,
                    after: 3,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helrld") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_code_points_clamps_at_doc_bounds() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 100,
                    after: 100,
                },
            },
        });
        // All text chars removed, paragraph remains empty
        use editor_schema::DocFlatExt;
        assert_eq!(editor.state().doc.flat_size(), 0);
    }

    #[test]
    fn surrounding_utf16_bmp_only() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Surrounding {
                    before: 2,
                    after: 3,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helrld") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_utf16_with_composition_preserves_composing() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 8)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 6, end: 11 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Surrounding {
                    before: 2,
                    after: 0,
                },
            },
        });
        // before_anchor = min(cursor=8, composition.start=6) = 6; delete (4, 6) = "o "
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hellworld") } } }
            selection: (t1, 6)  // cursor adjusted
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_with_composition_cursor_at_composition_start() {
        // cursor == before_anchor (= composition.start)
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 6)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 6, end: 11 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 2,
                    after: 0,
                },
            },
        });
        // before_anchor = min(6, 6) = 6; del_start = 4; delete [4, 6] = "o "
        // before_count = 6 - 4 = 2; new_cursor = 6 - 2 = 4
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hellworld") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_with_composition_cursor_before_composition() {
        // cursor < composition.start
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 6, end: 11 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 1,
                    after: 0,
                },
            },
        });
        // before_anchor = min(3, 6) = 3; del_start = 2; delete [2, 3] = "l"
        // before_count = 3 - 2 = 1; new_cursor = 3 - 1 = 2
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo world") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_with_composition_cursor_after_composition() {
        // cursor > composition.end
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab hello world") } } }
            selection: (t1, 12)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 3, end: 8 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 2,
                    after: 0,
                },
            },
        });
        // before_anchor = min(12, 3) = 3; del_start = 1; delete [1, 3] = "b "
        // before_count = 3 - 1 = 2; new_cursor = 12 - 2 = 10
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ahello world") } } }
            selection: (t1, 10)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_with_composition_after_only() {
        // cursor inside composition, after-only delete; before_count = 0 so cursor unchanged
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 3, end: 8 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 0,
                    after: 3,
                },
            },
        });
        // after_anchor = max(5, 8) = 8; delete [8, 11] = " wo"
        // before_count = 0; new_cursor = 5 - 0 = 5
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ab hellorld") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_with_composition_both_before_and_after() {
        // cursor inside composition, both deletes happen
        // "ab hello world!" indices (flat): a=0 b=1 ' '=2 h=3 e=4 l=5 l=6 o=7 ' '=8 w=9 o=10 r=11 l=12 d=13 !=14
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab hello world!") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 3, end: 8 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 2,
                    after: 3,
                },
            },
        });
        // before_anchor=3, after_anchor=8; del_start=1, del_end_after=11
        // delete [8,11]=" wo" first, then [1,3]="b "
        // result: "ahellorld!" (10 chars)
        // before_count = 3 - 1 = 2; new_cursor = 5 - 2 = 3
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ahellorld!") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward_combining_mark() {
        // "aéb" = a + e + U+0301 + b = 4 codepoints, 3 graphemes
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ae\u{0301}b") } } }
            selection: (t1, 3)
        };
        let resource = Arc::new(Mutex::new(Resource::new()));
        resource.lock().unwrap().segmenters = Some(Arc::new(TextSegmenters::new_test()));
        let mut editor = Editor::new_test_with_resource(state, resource);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn surrounding_no_delete_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::SurroundingCodePoints {
                    before: 0,
                    after: 0,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
