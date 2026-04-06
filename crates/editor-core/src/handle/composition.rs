use editor_commands::CommandError;
use editor_common::StrExt;
use editor_model::Doc;
use editor_schema::{DocFlatExt, FlatSegment, ResolvedPositionFlatExt};
use editor_state::Composition;
use editor_transaction::Transaction;

use super::helpers::replace_flat_range;
use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_composition_intent(
    editor: &mut Editor,
    intent: CompositionIntent,
) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match intent {
            CompositionIntent::SetRegion { start, end } => {
                let new_comp = composition_range_valid(&tr.doc(), start, end)
                    .then_some(Composition { start, end });
                tr.set_composition(new_comp)?;
            }
            CompositionIntent::CommitAsIs => {
                tr.set_composition(None)?;
            }
            CompositionIntent::Cancel => {
                if let Some(comp) = tr.composition().copied()
                    && composition_range_valid(&tr.doc(), comp.start, comp.end)
                {
                    replace_flat_range(tr, comp.start, comp.end, "")?;
                }
                tr.set_composition(None)?;
            }
            CompositionIntent::Update {
                text,
                replace_length,
            } => {
                let (target_start, target_end) = resolve_target(tr, replace_length)?;
                replace_flat_range(tr, target_start, target_end, &text)?;
                let new_end = target_start + text.char_count();
                tr.set_composition(Some(Composition {
                    start: target_start,
                    end: new_end,
                }))?;
            }
            CompositionIntent::Commit { text } => {
                let (target_start, target_end) = resolve_target(tr, None)?;
                replace_flat_range(tr, target_start, target_end, &text)?;
                tr.set_composition(None)?;
            }
        }
        Ok(())
    })
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
        if matches!(seg, FlatSegment::Break { .. } | FlatSegment::Atom { .. }) {
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
        // flat: "abc\ndef" → block boundary \n at index 3
        assert!(composition_range_valid(&state.doc, 0, 3));
        assert!(!composition_range_valid(&state.doc, 0, 4)); // crosses \n
        assert!(composition_range_valid(&state.doc, 4, 7));
    }

    #[test]
    fn composition_range_valid_rejects_out_of_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 0)
        };
        assert!(composition_range_valid(&state.doc, 0, 2));
        assert!(!composition_range_valid(&state.doc, 0, 3));
        assert!(!composition_range_valid(&state.doc, 2, 1)); // start > end
    }

    #[test]
    fn composition_range_valid_accepts_empty_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        // empty ranges (start == end) are valid anchors for composition
        assert!(composition_range_valid(&state.doc, 0, 0));
        assert!(composition_range_valid(&state.doc, 3, 3));
        assert!(composition_range_valid(&state.doc, 5, 5));
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
        // flat: "a\u{fffc}b" — a=0, img(atom)=1, b=2
        assert!(composition_range_valid(&state.doc, 0, 1)); // "a" only
        assert!(!composition_range_valid(&state.doc, 0, 2)); // crosses image atom
        assert!(!composition_range_valid(&state.doc, 1, 2)); // starts at image atom
        assert!(composition_range_valid(&state.doc, 2, 3)); // "b" only
    }

    #[test]
    fn set_region_stores_valid_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 1, end: 4 },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 4 },
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 5 },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 5 })
        );
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 6, end: 11 },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 6, end: 11 })
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 3 },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 3 })
        );
        // Now apply invalid cross-block range → should clear prior composition
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 5 },
            },
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
        // Range 0..2 crosses the image atom
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 2 },
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 3 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::CommitAsIs,
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 1, end: 4 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Cancel,
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Cancel,
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "X".into(),
                    replace_length: None,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("heXllo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn update_no_composition_replace_length_deletes_before() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "XY".into(),
                    replace_length: Some(2),
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYlo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 3 })
        );
    }

    #[test]
    fn update_with_composition_replaces_region() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 1, end: 4 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "XYZ".into(),
                    replace_length: None,
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYZo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "X".into(),
                    replace_length: None,
                },
            },
        });
        // resolve_target should detect stale composition, clear it,
        // and insert "X" at the cursor (flat offset 0).
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Xhi") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 1 })
        );
    }

    #[test]
    fn commit_with_composition_replaces_and_clears() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 1, end: 4 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Commit { text: "Y".into() },
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Commit { text: "!".into() },
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "한".into(),
                    replace_length: None,
                },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 1 })
        );
        // Replace with "안녕": 2 scalars, 2 flat offset units.
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "안녕".into(),
                    replace_length: None,
                },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 2 })
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 1, end: 4 },
            },
        });
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Commit { text: "".into() },
            },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Commit {
                    text: "안녕".into(),
                },
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
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Commit { text: "X".into() },
            },
        });
        // resolve_target detects stale composition, clears it, inserts "X" at cursor (flat 1).
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

        // IME: SetComposingRegion(0, 2) covers "안녕" (cross-node)
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::SetRegion { start: 0, end: 2 },
            },
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 2 })
        );

        // IME: Update("안녕하", None) — replace composing region with new text
        editor.apply(Message::Intent {
            intent: Intent::Composition {
                intent: CompositionIntent::Update {
                    text: "안녕하".into(),
                    replace_length: None,
                },
            },
        });

        // After: composition should be { start: 0, end: 3 }
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 3 })
        );

        assert_eq!(editor.state().doc.flat_text(0..3), "안녕하");
    }

    /// iOS Korean IME uses select-delete-reinsert instead of SetComposingText.
    /// Typing "안녕" after "!" produces:
    ///   Commit("ㅇ")
    ///   SetSelection(0,2) → Commit("") → Commit("!") → Commit("아")
    ///   SetSelection(0,2) → Commit("") → Commit("!") → Commit("안")
    ///   Commit("ㄴ")
    ///   SetSelection(1,3) → Commit("") → Commit("안") → Commit("녀")
    ///   SetSelection(1,3) → Commit("") → Commit("안") → Commit("녕")
    #[test]
    fn ios_korean_ime_select_delete_reinsert() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("!") } } }
            selection: (t1, 1)
        };
        let mut editor = Editor::new_test(state);

        let apply_commit = |e: &mut Editor, text: &str| {
            e.apply(Message::Intent {
                intent: Intent::Composition {
                    intent: CompositionIntent::Commit { text: text.into() },
                },
            });
        };
        let apply_set_selection = |e: &mut Editor, start: usize, end: usize| {
            e.apply(Message::Intent {
                intent: Intent::Selection {
                    intent: SelectionIntent::SetFlat { start, end },
                },
            });
        };

        // ㅇ
        apply_commit(&mut editor, "ㅇ");
        // SetSelection(0,2) → Commit("") → Commit("!") → Commit("아")
        apply_set_selection(&mut editor, 0, 2);
        apply_commit(&mut editor, "");
        apply_commit(&mut editor, "!");
        apply_commit(&mut editor, "아");
        // SetSelection(0,2) → Commit("") → Commit("!") → Commit("안")
        apply_set_selection(&mut editor, 0, 2);
        apply_commit(&mut editor, "");
        apply_commit(&mut editor, "!");
        apply_commit(&mut editor, "안");
        // ㄴ
        apply_commit(&mut editor, "ㄴ");
        // SetSelection(1,3) → Commit("") → Commit("안") → Commit("녀")
        apply_set_selection(&mut editor, 1, 3);
        apply_commit(&mut editor, "");
        apply_commit(&mut editor, "안");
        apply_commit(&mut editor, "녀");
        // SetSelection(1,3) → Commit("") → Commit("안") → Commit("녕")
        apply_set_selection(&mut editor, 1, 3);
        apply_commit(&mut editor, "");
        apply_commit(&mut editor, "안");
        apply_commit(&mut editor, "녕");

        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("!안녕") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
