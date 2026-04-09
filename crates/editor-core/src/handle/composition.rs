use editor_commands::{self as commands, CommandError, CommandResult};
use editor_common::StrExt;
use editor_model::Doc;
use editor_schema::{DocFlatExt, FlatSegment, ResolvedPositionFlatExt};
use editor_state::{Composition, ResolvedPosition, Selection};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_composition_op(editor: &mut Editor, op: CompositionOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match op {
            CompositionOp::SetRegion { start, end } => {
                let new_comp = composition_range_valid(&tr.doc(), start, end)
                    .then_some(Composition { start, end });
                tr.set_composition(new_comp)?;
            }
            CompositionOp::CommitAsIs => {
                tr.set_composition(None)?;
            }
            CompositionOp::Cancel => {
                if let Some(comp) = tr.composition().copied()
                    && composition_range_valid(&tr.doc(), comp.start, comp.end)
                {
                    replace_text_range(tr, comp.start, comp.end, "")?;
                }
                tr.set_composition(None)?;
            }
            CompositionOp::Update {
                text,
                replace_length,
            } => {
                let (target_start, target_end) = resolve_target(tr, replace_length)?;
                replace_text_range(tr, target_start, target_end, &text)?;
                let new_end = target_start + text.char_count();
                tr.set_composition(Some(Composition {
                    start: target_start,
                    end: new_end,
                }))?;
            }
            CompositionOp::Commit { text } => {
                let (target_start, target_end) = resolve_target(tr, None)?;
                replace_text_range(tr, target_start, target_end, &text)?;
                tr.set_composition(None)?;
            }
        }
        Ok(())
    })
}

fn replace_text_range(tr: &mut Transaction, start: usize, end: usize, text: &str) -> CommandResult {
    let doc = tr.doc();
    let start_pos = ResolvedPosition::from_flat(&doc, start)
        .ok_or(CommandError::Corrupted("flat start unresolvable".into()))?;
    let end_pos = ResolvedPosition::from_flat(&doc, end)
        .ok_or(CommandError::Corrupted("flat end unresolvable".into()))?;

    commands::chain!(
        tr,
        commands::set_selection(Selection::new((&start_pos).into(), (&end_pos).into())),
        commands::when!(start != end, commands::delete_selection()),
        commands::when!(!text.is_empty(), commands::insert_text(text)),
    )
}

fn composition_range_valid(doc: &Doc, start: usize, end: usize) -> bool {
    if start > end || end > doc.flat_size() {
        return false;
    }
    for (seg_start, seg) in doc.flat_segments() {
        if seg_start >= end {
            break;
        }
        if seg_start < start {
            continue;
        }
        if matches!(
            seg,
            FlatSegment::Break { .. }
                | FlatSegment::Atom { .. }
                | FlatSegment::Open { .. }
                | FlatSegment::Close { .. }
        ) {
            return false;
        }
    }
    true
}

fn resolve_target(
    tr: &mut Transaction,
    replace_length: Option<usize>,
) -> Result<(usize, usize), CommandError> {
    let doc = tr.doc();

    if let Some(comp) = tr.composition().copied() {
        if composition_range_valid(&doc, comp.start, comp.end) {
            return Ok((comp.start, comp.end));
        }
        tr.set_composition(None)?;
    }

    let sel = tr.selection();

    if !sel.is_collapsed() {
        let anchor_flat = sel
            .anchor
            .resolve(&doc)
            .ok_or(CommandError::Corrupted("anchor unresolvable".into()))?
            .to_flat();
        let head_flat = sel
            .head
            .resolve(&doc)
            .ok_or(CommandError::Corrupted("head unresolvable".into()))?
            .to_flat();
        return Ok((anchor_flat.min(head_flat), anchor_flat.max(head_flat)));
    }

    let cursor_flat = sel
        .head
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cursor unresolvable".into()))?
        .to_flat();

    if let Some(len) = replace_length {
        Ok((cursor_flat.saturating_sub(len), cursor_flat))
    } else {
        Ok((cursor_flat, cursor_flat))
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn composition_range_valid_rejects_cross_block() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (t1, 0)
        };
        // O(p1)=0, abc=1..4, C(p1)=4, O(p2)=5, def=6..9, C(p2)=9
        assert!(composition_range_valid(&state.doc, 1, 4));
        assert!(!composition_range_valid(&state.doc, 1, 5)); // crosses C(p1)
        assert!(composition_range_valid(&state.doc, 6, 9));
    }

    #[test]
    fn composition_range_valid_rejects_out_of_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 0)
        };
        // O=0, ab=1..3, C=3; flat_size=4
        assert!(composition_range_valid(&state.doc, 1, 3));
        assert!(!composition_range_valid(&state.doc, 1, 4));
        assert!(!composition_range_valid(&state.doc, 2, 1)); // start > end
    }

    #[test]
    fn composition_range_valid_accepts_empty_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        // O=0, hello=1..6, C=6; empty ranges are valid anchors for composition
        assert!(composition_range_valid(&state.doc, 1, 1));
        assert!(composition_range_valid(&state.doc, 4, 4));
        assert!(composition_range_valid(&state.doc, 6, 6));
    }

    #[test]
    fn composition_range_valid_rejects_atom() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") image {} text("b") }
                }
            }
            selection: (t1, 0)
        };
        // O=0, a=1, img=2, b=3, C=4
        assert!(composition_range_valid(&state.doc, 1, 2)); // "a" only
        assert!(!composition_range_valid(&state.doc, 1, 3)); // crosses image atom
        assert!(!composition_range_valid(&state.doc, 2, 3)); // starts at image atom
        assert!(composition_range_valid(&state.doc, 3, 4)); // "b" only
    }

    #[test]
    fn composition_range_valid_rejects_open_token() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        // O=0, text=1..6, C=6
        assert!(!composition_range_valid(&state.doc, 0, 2)); // includes Open
        assert!(!composition_range_valid(&state.doc, 5, 7)); // includes Close
        assert!(composition_range_valid(&state.doc, 1, 6)); // text only
    }

    #[test]
    fn composition_range_valid_rejects_nested_tokens() {
        let (state, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("hi") } } } }
            selection: (t1, 0)
        };
        // O(bq)=0, O(p)=1, h=2, i=3, C(p)=4, C(bq)=5
        assert!(composition_range_valid(&state.doc, 2, 4)); // "hi"
        assert!(!composition_range_valid(&state.doc, 1, 4)); // includes Open(p)
        assert!(!composition_range_valid(&state.doc, 2, 5)); // includes Close(p)
    }

    #[test]
    fn set_region_stores_valid_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 2, end: 5 },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 5 })
        );
    }

    #[test]
    fn set_region_rejects_cross_block() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("abc") }
                    paragraph { t2: text("def") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 6 },
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_region_replaces_prior_composition() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 6 },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 6 })
        );
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 7, end: 12 },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 7, end: 12 })
        );
    }

    #[test]
    fn set_region_invalid_clears_prior_composition() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 4 },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
        );
        // Now apply invalid cross-block range → should clear prior composition
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 6 },
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_region_rejects_atom_via_apply() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") image {} text("b") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Range 1..3 crosses the image atom
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 3 },
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_as_is_clears_composition() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 4 },
        });
        editor.apply(Message::Composition {
            op: CompositionOp::CommitAsIs,
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cancel_deletes_composing_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 2, end: 5 },
        });
        editor.apply(Message::Composition {
            op: CompositionOp::Cancel,
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ho") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cancel_without_composition_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Cancel,
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "X".into(),
                replace_length: None,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("heXllo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 3, end: 4 })
        );
    }

    #[test]
    fn update_no_composition_replace_length_deletes_before() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "XY".into(),
                replace_length: Some(2),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYlo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 4 })
        );
    }

    #[test]
    fn update_with_composition_replaces_region() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 2, end: 5 },
        });
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "XYZ".into(),
                replace_length: None,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYZo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 5 })
        );
    }

    #[test]
    fn update_stale_composition_falls_back_to_cursor() {
        let (mut state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        // Manually inject a stale composition (range exceeds doc size of 2).
        state.composition = Some(Composition { start: 10, end: 20 });
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "X".into(),
                replace_length: None,
            },
        });
        // resolve_target should detect stale composition, clear it,
        // and insert "X" at the cursor (flat offset 1).
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Xhi") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn commit_with_composition_replaces_and_clears() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 2, end: 5 },
        });
        editor.apply(Message::Composition {
            op: CompositionOp::Commit { text: "Y".into() },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hYo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Commit { text: "!".into() },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hi!") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Type "한" (Korean "Han"): single Unicode scalar, 3 UTF-8 bytes, 1 flat offset unit.
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "한".into(),
                replace_length: None,
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
        // Replace with "안녕": 2 scalars, 2 flat offset units.
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "안녕".into(),
                replace_length: None,
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 3 })
        );
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("안녕") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn commit_empty_text_deletes_composition_region() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 2, end: 5 },
        });
        editor.apply(Message::Composition {
            op: CompositionOp::Commit { text: "".into() },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ho") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Commit {
                text: "안녕".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hi안녕") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_stale_composition_falls_back_to_cursor() {
        let (mut state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 1)
        };
        // Inject stale composition (range exceeds doc size of 2).
        state.composition = Some(Composition { start: 10, end: 20 });
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Composition {
            op: CompositionOp::Commit { text: "X".into() },
        });
        // resolve_target detects stale composition, clears it, inserts "X" at cursor (flat 2).
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXi") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn retroactive_composition_across_formatting_boundary() {
        // "안"[bold] + "녕" (two text nodes in same paragraph, different modifiers)
        let (state, ..) = state! {
            doc { root { paragraph {
                text("안") [bold]
                t2: text("녕")
            }}}
            selection: (t2, 1)  // cursor after "녕"
        };
        let mut editor = Editor::new_test(state);

        // IME: SetComposingRegion(1, 3) covers "안녕" (cross-node)
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion { start: 1, end: 3 },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 3 })
        );

        // IME: Update("안녕하", None) — replace composing region with new text
        editor.apply(Message::Composition {
            op: CompositionOp::Update {
                text: "안녕하".into(),
                replace_length: None,
            },
        });

        // After: composition should be { start: 1, end: 4 }
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
        );

        assert_eq!(editor.state().doc.flat_text(1..4), "안녕하");
    }
}
