use std::sync::{Arc, Mutex};

use editor_macros::state;
use editor_model::Modifier;
use editor_resource::{
    RawTextReplacementRule, Resource, ResourceSource, prepare_text_replacement_rules,
};
use editor_state::{
    Composition, PendingModifier, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
    State, assert_state_eq,
};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::message::*;

fn rule(pattern: &str, sub: &str, regex: bool) -> RawTextReplacementRule {
    RawTextReplacementRule {
        id: pattern.into(),
        match_pattern: pattern.into(),
        substitute: sub.into(),
        regex,
    }
}

fn editor_with_rules(state: State, rules: Vec<RawTextReplacementRule>) -> Editor {
    let mut source = ResourceSource::new_test();
    source
        .set_text_replacement_rules(prepare_text_replacement_rules(rules))
        .expect("text replacement rules must change resources");
    let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
    Editor::new_test_with_resource(state, resource)
}

fn type_text(editor: &mut Editor, text: &str) {
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: text.into() },
    });
}

fn key(editor: &mut Editor, k: Key) {
    editor.apply(Message::Key {
        event: KeyEvent {
            key: k,
            modifiers: InputModifiers::default(),
        },
    });
}

fn flat_text(editor: &Editor) -> String {
    let view = editor.state().view();
    editor_state::flat_text(&view, 0..editor_state::flat_size(&view))
}

fn caret_flat(editor: &Editor) -> usize {
    let view = editor.state().view();
    editor
        .state()
        .selection
        .expect("selection must exist")
        .head
        .resolve(&view)
        .map(|rp| rp.to_flat())
        .expect("cursor must resolve")
}

const PLAIN_PATTERN: &str = "abc";
const PLAIN_SUBSTITUTE: &str = "X";
const REGEX_PATTERN: &str = r"\d+#";
const REGEX_SUBSTITUTE: &str = "N";
const MULTILINE_SUB_PATTERN: &str = "mk";
const MULTILINE_SUB_SUBSTITUTE: &str = "a\nb";
const MULTILINE_PAT_PATTERN: &str = r"P1\nP2";
const MULTILINE_PAT_SUBSTITUTE: &str = "Q";

#[test]
fn ime_leading_newline_preserves_list_range_enter_behavior() {
    for text in ["\n", "\r\n"] {
        let (initial, ..) = state! {
            doc { root { bullet_list { list_item { p: paragraph { text("abc") } } } paragraph {} } }
            selection: (p, 0) -> (p, 3)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: text.into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph {} }
                    list_item { p: paragraph {} }
                }
                paragraph {}
            } }
            selection: (p, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_leading_newline_inserts_only_one_paragraph_after_a_unit_selection() {
    for text in ["\n", "\r\n"] {
        let (initial, ..) = state! {
            doc { r: root { paragraph { text("a") } horizontal_rule paragraph { text("b") } } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: text.into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } horizontal_rule p: paragraph {} paragraph { text("b") } } }
            selection: (p, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_leading_newline_materializes_a_gap_without_splitting_it_again() {
    for text in ["\n", "\r\n"] {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: text.into() }],
        });
        let (expected, ..) = state! {
            doc { root { p: paragraph {} image paragraph { text("b") } } }
            selection: (p, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_multiline_commit_in_fold_title_keeps_text_when_enter_is_inapplicable() {
    for ops in [
        vec![FlatImeOp::ReplaceSelection {
            text: "X\nY".into(),
        }],
        vec![FlatImeOp::ReplaceSelection {
            text: "X\r\nY".into(),
        }],
        vec![
            FlatImeOp::ReplaceSelection { text: "\n".into() },
            FlatImeOp::CommitAsIs,
            FlatImeOp::SetSelection { start: 4, end: 4 },
            FlatImeOp::ReplaceSelection { text: "XY".into() },
        ],
    ] {
        let (initial, ..) = state! {
            doc { root {
                fold { title: fold_title { text("ab") } fold_content { paragraph {} } }
                paragraph {}
            } }
            selection: (title, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput { ops });
        let (expected, ..) = state! {
            doc { root {
                fold { title: fold_title { text("aXYb") } fold_content { paragraph {} } }
                paragraph {}
            } }
            selection: (title, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_leading_gap_break_binds_all_input_boundaries_to_the_new_paragraph() {
    for offset in 0..=2 {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection {
                    text: "\r\n".into(),
                },
                FlatImeOp::CommitAsIs,
                FlatImeOp::SetSelection {
                    start: offset,
                    end: offset,
                },
                FlatImeOp::ReplaceSelection { text: "X".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p: paragraph { text("X") } image paragraph { text("b") } } }
            selection: (p, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_crlf_partial_delete_updates_coordinates_even_without_a_document_edit() {
    for delete in [
        FlatImeOp::DeleteSurrounding {
            before: 1,
            after: 0,
        },
        FlatImeOp::DeleteSurroundingUtf16 {
            before: 1,
            after: 0,
        },
    ] {
        let (initial, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection {
                    text: "a\r\nb".into(),
                },
                FlatImeOp::CommitAsIs,
                FlatImeOp::SetSelection { start: 4, end: 4 },
                delete,
                // The input is now OPEN a CR b CLOSE: offset 4 is after b.
                FlatImeOp::SetSelection { start: 4, end: 4 },
                FlatImeOp::ReplaceSelection { text: "X".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } p: paragraph { text("bX") } } }
            selection: (p, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_disjoint_edits_keep_automatic_replacement_undo_boundary() {
    let (initial, ..) = state! {
        doc { root { p: paragraph { text("abcdef") } } }
        selection: (p, 1) -> (p, 2)
    };
    let mut editor = editor_with_rules(initial, vec![rule("B", "β", false)]);
    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::ReplaceSelection { text: "B".into() },
            FlatImeOp::SetSelection { start: 5, end: 6 },
            FlatImeOp::ReplaceSelection { text: "E".into() },
        ],
    });
    assert_eq!(flat_text(&editor), "\u{2028}aβcdEf\u{2029}");
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(flat_text(&editor), "\u{2028}aβcdef\u{2029}");
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(flat_text(&editor), "\u{2028}aBcdef\u{2029}");
}

#[test]
fn ime_deletion_and_empty_composition_match_the_native_input_buffer() {
    let fixtures: std::collections::BTreeMap<String, Vec<Message>> =
        serde_json::from_str(include_str!("tests/fixtures/ime-delete-composition.json")).unwrap();
    for (name, start, end, composition, text, caret) in [
        ("delete-before-selection", 2, 4, None, "Xd", 1),
        ("delete-inside-composition", 3, 3, Some((2, 4)), "aXd", 2),
        ("empty-composition", 3, 3, Some((2, 3)), "acXd", 3),
    ] {
        let (initial, ..) = state! {
            doc { root { p: paragraph { text("abcd") } } }
            selection: (p, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetSelection { start, end }],
        });
        if let Some((start, end)) = composition {
            editor.apply(Message::TextInput {
                ops: vec![FlatImeOp::SetComposition { start, end }],
            });
        }
        let request = editor.enqueue_request(fixtures[name].clone()).unwrap();
        editor.tick_through(request).unwrap();
        assert_eq!(
            flat_text(&editor),
            format!("\u{2028}{text}\u{2029}"),
            "{name}"
        );
        assert_eq!(caret_flat(&editor), caret + 1, "{name}");
        assert_eq!(editor.state().composition, None, "{name}");
    }
}

#[test]
fn ime_multiline_selection_matches_the_native_input_buffer() {
    let (initial, ..) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = Editor::new_test(initial);
    // Also checked against the real Kotlin normalizer. Native text starts as
    // OPEN CLOSE, then becomes OPEN a newline b CLOSE; offset 4 is after b.
    let messages: Vec<Message> =
        serde_json::from_str(include_str!("tests/fixtures/ime-multiline-selection.json")).unwrap();
    let request = editor.enqueue_request(messages).unwrap();
    editor.tick_through(request).unwrap();

    let (expected, ..) = state! {
        doc { root { paragraph { text("a") } p: paragraph { text("bX") } } }
        selection: (p, 2)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_multiline_positions_survive_commit_barriers_and_later_edits() {
    for (text, end) in [("a\nb", 4), ("a\r\nb", 5), ("a\rb", 4)] {
        let (initial, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test(initial);
        let request = editor
            .enqueue_request(vec![Message::TextInput {
                ops: vec![
                    FlatImeOp::Compose { text: text.into() },
                    FlatImeOp::CommitAsIs,
                    FlatImeOp::ReplaceSelection {
                        text: "😀".into()
                    },
                    FlatImeOp::SetSelection {
                        start: end + 1,
                        end: end + 1,
                    },
                    FlatImeOp::ReplaceSelection { text: "X".into() },
                ],
            }])
            .unwrap();
        editor.tick_through(request).unwrap();
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } p: paragraph { text("b😀X") } } }
            selection: (p, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

#[test]
fn ime_multiline_composition_maps_to_the_inserted_paragraph() {
    let (initial, ..) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = Editor::new_test(initial);
    let request = editor
        .enqueue_request(vec![Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection {
                    text: "a\nb".into(),
                },
                FlatImeOp::CommitAsIs,
                FlatImeOp::SetComposition { start: 3, end: 4 },
                FlatImeOp::Compose { text: "乙".into() },
                FlatImeOp::SetSelection { start: 3, end: 3 },
            ],
        }])
        .unwrap();
    editor.tick_through(request).unwrap();
    let (expected, ..) = state! {
        doc { root { paragraph { text("a") } p: paragraph { text("乙") } } }
        selection: (p, 0)
    };
    assert_state_eq!(editor.state(), &expected);
    assert_eq!(
        editor.state().composition,
        Some(Composition { start: 4, end: 5 })
    );
}

#[test]
fn ime_delete_after_multiline_commit_counts_input_newlines_not_structural_tokens() {
    let (initial, ..) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = Editor::new_test(initial);
    let request = editor
        .enqueue_request(vec![Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection {
                    text: "a\nb".into(),
                },
                FlatImeOp::CommitAsIs,
                FlatImeOp::SetSelection { start: 3, end: 3 },
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::ReplaceSelection { text: "X".into() },
            ],
        }])
        .unwrap();
    editor.tick_through(request).unwrap();
    let (expected, ..) = state! {
        doc { root { p: paragraph { text("aXb") } } }
        selection: (p, 2)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_messages_start_in_document_coordinates_even_in_one_request() {
    let (initial, ..) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = Editor::new_test(initial);
    let request = editor
        .enqueue_request(vec![
            Message::TextInput {
                ops: vec![FlatImeOp::ReplaceSelection {
                    text: "a\nb".into(),
                }],
            },
            Message::TextInput {
                ops: vec![
                    FlatImeOp::SetSelection { start: 5, end: 5 },
                    FlatImeOp::ReplaceSelection { text: "X".into() },
                ],
            },
        ])
        .unwrap();
    editor.tick_through(request).unwrap();
    let (expected, ..) = state! {
        doc { root { paragraph { text("a") } p: paragraph { text("bX") } } }
        selection: (p, 2)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_position_after_commit_follows_automatic_replacement() {
    let (initial, ..) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = editor_with_rules(initial, vec![rule("abc", "X", false)]);
    let request = editor
        .enqueue_request(vec![Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "abc".into() },
                FlatImeOp::CommitAsIs,
                FlatImeOp::SetSelection { start: 4, end: 4 },
                FlatImeOp::ReplaceSelection { text: "!".into() },
            ],
        }])
        .unwrap();
    editor.tick_through(request).unwrap();
    let (expected, ..) = state! {
        doc { root { p: paragraph { text("X!") } } }
        selection: (p, 2)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn plain_rule_applies_on_insertion() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    for ch in PLAIN_PATTERN.chars() {
        type_text(&mut editor, &ch.to_string());
    }

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("X") } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn no_rule_means_no_replacement() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);

    type_text(&mut editor, PLAIN_PATTERN);

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn non_matching_input_is_untouched() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, "zzz");

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("zzz") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn regex_rule_applies_on_insertion() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(REGEX_PATTERN, REGEX_SUBSTITUTE, true)]);

    type_text(&mut editor, "42#");

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("N") } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn multiline_substitute_creates_hard_break() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(
        s,
        vec![rule(MULTILINE_SUB_PATTERN, MULTILINE_SUB_SUBSTITUTE, false)],
    );

    type_text(&mut editor, MULTILINE_SUB_PATTERN);

    let flat = flat_text(&editor);
    for segment in MULTILINE_SUB_SUBSTITUTE.split('\n') {
        assert!(
            flat.contains(segment),
            "missing substitute segment {segment:?} in {flat:?}"
        );
    }
    assert!(
        !flat.contains(MULTILINE_SUB_PATTERN),
        "pattern leftover in {flat:?}"
    );
}

#[test]
fn multiline_pattern_matches_across_hard_break() {
    let (s, ..) = state! {
        doc {
            root {
                _p1: paragraph {
                    text("P1")
                    hard_break {}
                    text("P")
                }
            }
        }
        selection: (_p1, 4)
    };
    let mut editor = editor_with_rules(
        s,
        vec![rule(MULTILINE_PAT_PATTERN, MULTILINE_PAT_SUBSTITUTE, true)],
    );

    type_text(&mut editor, "2");

    let flat = flat_text(&editor);
    assert!(flat.contains(MULTILINE_PAT_SUBSTITUTE), "got: {flat:?}");
    assert!(!flat.contains("P1"), "got: {flat:?}");
    assert!(!flat.contains("P2"), "got: {flat:?}");
}

#[test]
fn pattern_does_not_match_across_tab_atom() {
    let (s, ..) = state! {
        doc {
            root {
                p1: paragraph {
                    text("a")
                    tab
                    text("b")
                }
            }
        }
        selection: (p1, 3)
    };
    let mut editor = editor_with_rules(s, vec![rule("abz", "X", false)]);

    type_text(&mut editor, "z");

    let (expected, ..) = state! {
        doc {
            root {
                p1: paragraph {
                    text("a")
                    tab
                    text("bz")
                }
            }
        }
        selection: (p1, 4)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn backspace_immediately_after_replacement_restores_original() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    key(&mut editor, Key::Backspace);

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn backspace_restore_clears_pending_format_restored_by_auto_replacement_undo() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);
    editor
        .transact(|tr| {
            tr.set_pending_modifiers(vec![PendingModifier::Set {
                modifier: Modifier::Bold,
            }])?;
            Ok(())
        })
        .unwrap();

    type_text(&mut editor, PLAIN_PATTERN);
    key(&mut editor, Key::Backspace);

    let flat = flat_text(&editor);
    assert!(
        flat.contains(PLAIN_PATTERN),
        "original text must be restored by shortcut: {flat:?}"
    );
    assert!(
        !flat.contains(PLAIN_SUBSTITUTE),
        "substitute must be gone: {flat:?}"
    );
    assert!(
        editor.state().pending_modifiers.is_empty(),
        "pending modifiers cleared"
    );
    assert!(editor.undo_history.can_redo(), "redo stack must be intact");
}

#[test]
fn second_backspace_after_restore_is_normal_delete() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    key(&mut editor, Key::Backspace);
    key(&mut editor, Key::Backspace);

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("ab") } } }
        selection: (p1, 2)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_backspace_batch_immediately_after_replacement_restores_original() {
    // iOS soft-keyboard backspace arrives through the flat IME path as a
    // select-last-grapheme + empty-commit batch, not as a Backspace key event.
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    let caret = caret_flat(&editor);
    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::SetSelection {
                start: caret - 1,
                end: caret,
            },
            FlatImeOp::Compose { text: "".into() },
            FlatImeOp::CommitAsIs,
        ],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_delete_surrounding_immediately_after_replacement_restores_original() {
    // Android soft-keyboard backspace arrives as deleteSurroundingText(1, 0).
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::DeleteSurrounding {
            before: 1,
            after: 0,
        }],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn ime_multi_grapheme_backward_delete_after_replacement_is_normal_delete() {
    // Only a single-grapheme backward delete is a backspace; a wider batch
    // delete (e.g. word delete) must not trigger the restore.
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule("ab", "XY", false)]);

    for ch in "ab".chars() {
        type_text(&mut editor, &ch.to_string());
    }
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::DeleteSurrounding {
            before: 2,
            after: 0,
        }],
    });

    let flat = flat_text(&editor);
    assert!(!flat.contains("ab"), "restore must not fire: {flat:?}");
    assert!(
        !flat.contains("XY"),
        "substitute must be deleted normally: {flat:?}"
    );
}

#[test]
fn typing_after_replacement_invalidates_restore() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    type_text(&mut editor, "z");
    key(&mut editor, Key::Backspace);

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("X") } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn cursor_movement_invalidates_restore() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("k") } } }
        selection: (p1, 1)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);

    // In tests, view-based movement (move_grapheme) is a no-op because the view
    // has no layout. Push a SetSelection step with a genuinely different cursor
    // position so history reflects that the user moved the cursor, exactly as
    // navigation would in production. Navigation uses HistoryMeta::Skip so that
    // selection-only moves are not undoable and also clear the last_tag (preventing
    // shortcut restore). Move left one char then immediately back so the cursor ends
    // up at the original position for the subsequent Backspace assertion.
    let current_sel = editor
        .state()
        .selection
        .expect("selection must exist after typing");
    let current_flat = {
        let view = editor.state().view();
        current_sel
            .head
            .resolve(&view)
            .map(|rp| rp.to_flat())
            .expect("cursor must resolve")
    };
    let left_sel = {
        let view = editor.state().view();
        let pos = ResolvedPosition::from_flat(&view, current_flat - 1)
            .expect("flat pos left of cursor resolves");
        Selection::collapsed(Position::from(&pos))
    };
    editor
        .transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            tr.set_selection(Some(left_sel))?;
            Ok(())
        })
        .unwrap();
    editor
        .transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            tr.set_selection(Some(current_sel))?;
            Ok(())
        })
        .unwrap();

    key(&mut editor, Key::Backspace);

    let flat = flat_text(&editor);
    assert!(
        !flat.contains(PLAIN_PATTERN),
        "restore must not fire after navigation: {flat:?}"
    );
    assert!(
        !flat.contains(PLAIN_SUBSTITUTE),
        "substitute must be gone after normal backspace: {flat:?}"
    );
    assert!(
        flat.contains('k'),
        "pre-existing text must remain: {flat:?}"
    );
}

#[test]
fn multiline_replacement_restored_by_backspace() {
    let (s, ..) = state! {
        doc {
            root {
                _p1: paragraph {
                    text("P1")
                    hard_break {}
                    text("P")
                }
            }
        }
        selection: (_p1, 4)
    };
    let mut editor = editor_with_rules(
        s,
        vec![rule(MULTILINE_PAT_PATTERN, MULTILINE_PAT_SUBSTITUTE, true)],
    );

    type_text(&mut editor, "2");
    key(&mut editor, Key::Backspace);

    let flat = flat_text(&editor);
    assert!(flat.contains("P1"), "got: {flat:?}");
    assert!(flat.contains("P2"), "got: {flat:?}");
    assert!(
        !flat.contains(MULTILINE_PAT_SUBSTITUTE),
        "substitute must be gone: {flat:?}"
    );
}

#[test]
fn undo_after_replacement_does_not_leave_substitute_in_doc() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    let flat = flat_text(&editor);
    assert!(
        !flat.contains(PLAIN_SUBSTITUTE),
        "undo must rewind through the replacement: {flat:?}"
    );
}

#[test]
fn redo_after_undo_restores_substitute() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });

    let flat = flat_text(&editor);
    assert!(
        flat.contains(PLAIN_SUBSTITUTE),
        "redo must reproduce the replacement: {flat:?}"
    );
}

#[test]
fn undo_then_backspace_is_safe_when_replacement_undo_state_was_live() {
    // After undo+redo, last_tag() is still AutoReplacement, so backspace fires
    // the shortcut again and restores the original text. The mechanism must not
    // corrupt the history stack.
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    type_text(&mut editor, PLAIN_PATTERN);
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });
    key(&mut editor, Key::Backspace);

    // Backspace shortcut fires (last_tag == AutoReplacement after redo),
    // undoing the replacement and restoring the original text.
    let flat = flat_text(&editor);
    assert!(
        !flat.contains(PLAIN_SUBSTITUTE),
        "substitute must be gone: {flat:?}"
    );
    assert!(
        flat.contains(PLAIN_PATTERN),
        "original text must be restored by shortcut: {flat:?}"
    );
    assert!(editor.undo_history.can_redo(), "redo stack must be intact");
}

#[test]
fn replacement_skipped_during_active_composition() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose {
            text: PLAIN_PATTERN.into(),
        }],
    });

    let flat = flat_text(&editor);
    assert!(
        flat.contains(PLAIN_PATTERN),
        "raw composing text must remain in doc: {flat:?}"
    );
    assert!(
        !flat.contains(PLAIN_SUBSTITUTE),
        "replacement must not fire mid-composition: {flat:?}"
    );
    assert!(
        editor.state().composition.is_some(),
        "composition should still be active"
    );
}

#[test]
fn replacement_fires_on_commit_as_is() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    // CommitAsIs is the path the web host takes on `compositionend`.
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose {
            text: PLAIN_PATTERN.into(),
        }],
    });
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::CommitAsIs],
    });

    let flat = flat_text(&editor);
    assert!(
        flat.contains(PLAIN_SUBSTITUTE),
        "commit must trigger replacement: {flat:?}"
    );
    assert!(
        !flat.contains(PLAIN_PATTERN),
        "raw composing text must be replaced: {flat:?}"
    );
    assert!(editor.state().composition.is_none());
}

#[test]
fn commit_after_cursor_move_does_not_replace_destination_text() {
    let (s, p1) = state! {
        doc { root { p1: paragraph { text("ㅎㅎ 본문") } } }
        selection: (p1, 5)
    };
    let mut editor = editor_with_rules(s, vec![rule("ㅎㅎ", "웃음", false)]);
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose { text: "한".into() }],
    });
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(p1, 2)),
        },
    });
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::CommitAsIs],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("ㅎㅎ 본문한") } } }
        selection: (p1, 2)
    };
    assert_state_eq!(editor.state(), &expected);
    assert!(editor.state().composition.is_none());

    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose { text: "ㄱ".into() }],
    });
    assert_eq!(flat_text(&editor), "\u{2028}ㅎㅎㄱ 본문한\u{2029}");
    assert_eq!(caret_flat(&editor), 4);
    assert_eq!(
        editor.state().composition,
        Some(Composition { start: 3, end: 4 })
    );
}

#[test]
fn commit_after_cursor_move_replaces_at_composition_and_preserves_selection() {
    for (anchor, head, expected_anchor, expected_head) in [(2, 2, 2, 2), (6, 6, 7, 7), (6, 4, 7, 5)]
    {
        let (s, p1) = state! {
            doc { root { p1: paragraph { text("ㅎㅎ 본문") } } }
            selection: (p1, 3)
        };
        let mut editor = editor_with_rules(
            s,
            vec![rule("ㅎㅎ", "웃음", false), rule("한", "한글", false)],
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "한".into() }],
        });
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::new(Position::new(p1, anchor), Position::new(p1, head)),
            },
        });
        let moved_selection = editor.state().selection;
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });

        assert_eq!(flat_text(&editor), "\u{2028}ㅎㅎ 한글본문\u{2029}");
        assert_eq!(
            editor.state().selection,
            Selection::new(
                Position::new(p1, expected_anchor),
                Position::new(p1, expected_head),
            )
            .normalize(&editor.state().view()),
        );
        assert!(editor.state().composition.is_none());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_eq!(flat_text(&editor), "\u{2028}ㅎㅎ 한본문\u{2029}");
        assert_eq!(editor.state().selection, moved_selection);
        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });

        // Backspace acts on the moved selection, not on the replacement at the old composition.
        key(&mut editor, Key::Backspace);
        assert_eq!(
            flat_text(&editor),
            match (anchor, head) {
                (2, 2) => "\u{2028}ㅎ 한글본문\u{2029}",
                (6, 6) => "\u{2028}ㅎㅎ 한글본\u{2029}",
                _ => "\u{2028}ㅎㅎ 한글\u{2029}",
            },
        );
    }
}

#[test]
fn commit_barrier_replaces_before_later_op_in_same_text_input() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule("ㅠㅠ", "하하하", false)]);

    type_text(&mut editor, "ㅠ");
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose { text: "ㅠ".into() }],
    });
    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::CommitAsIs,
            FlatImeOp::ReplaceSelection { text: " ".into() },
        ],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("하하하 ") } } }
        selection: (p1, 4)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn commit_barrier_matches_split_delivery_including_undo() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let rules = vec![rule("ㅠㅠ", "하하하", false)];
    let mut batched = editor_with_rules(s.clone(), rules.clone());
    let mut split = editor_with_rules(s, rules);

    for editor in [&mut batched, &mut split] {
        type_text(editor, "ㅠ");
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅠ".into() }],
        });
    }
    batched.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::CommitAsIs,
            FlatImeOp::ReplaceSelection { text: " ".into() },
        ],
    });
    split.apply(Message::TextInput {
        ops: vec![FlatImeOp::CommitAsIs],
    });
    split.apply(Message::TextInput {
        ops: vec![FlatImeOp::ReplaceSelection { text: " ".into() }],
    });

    assert_state_eq!(batched.state(), split.state());
    for editor in [&mut batched, &mut split] {
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
    }
    assert_state_eq!(batched.state(), split.state());
}

#[test]
fn commit_as_is_without_active_composition_does_not_trigger_replacement() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    let mut editor = editor_with_rules(s, vec![rule("abc", "X", false)]);

    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::CommitAsIs],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn empty_compose_commit_without_active_composition_does_not_trigger_replacement() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    let mut editor = editor_with_rules(s, vec![rule("abc", "X", false)]);

    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::Compose {
                text: String::new(),
            },
            FlatImeOp::CommitAsIs,
        ],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn commit_after_clear_composition_does_not_trigger_replacement() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    let mut editor = editor_with_rules(s, vec![rule("abc", "X", false)]);
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::SetComposition { start: 1, end: 4 }],
    });

    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::ClearComposition, FlatImeOp::CommitAsIs],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
    assert!(editor.state().composition.is_none());
}

#[test]
fn replacement_fires_before_following_text_input_in_same_request() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule("ㅜㅜ", "ㅋㅋ", false)]);

    type_text(&mut editor, "ㅜ");
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose { text: "ㅜ".into() }],
    });
    editor
        .enqueue_request(vec![
            Message::TextInput {
                ops: vec![FlatImeOp::CommitAsIs],
            },
            Message::TextInput {
                ops: vec![
                    FlatImeOp::Compose { text: " ".into() },
                    FlatImeOp::CommitAsIs,
                ],
            },
        ])
        .unwrap();
    editor.tick().unwrap();

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("ㅋㅋ ") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn movement_after_replacement_uses_the_replaced_document_layout() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("가가") } } }
        selection: (p1, 2)
    };
    let mut editor = editor_with_rules(s, vec![rule("ㅠㅠ", "하하하", false)]);

    type_text(&mut editor, "ㅠ");
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose { text: "ㅠ".into() }],
    });
    editor
        .enqueue_request(vec![
            Message::TextInput {
                ops: vec![FlatImeOp::CommitAsIs],
            },
            Message::Navigation {
                op: NavigationOp::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                    extend: false,
                },
            },
        ])
        .unwrap();
    editor.tick().unwrap();

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("가가하하하") } } }
        selection: (p1, 5)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn flat_text_input_message_commits_preedit() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    let message: Message = serde_json::from_value(serde_json::json!({
        "type": "text_input",
        "ops": [
            { "type": "compose", "text": PLAIN_PATTERN },
            { "type": "commit_as_is" }
        ]
    }))
    .expect("flat text input message should deserialize");
    editor.apply(message);

    let flat = flat_text(&editor);
    assert!(flat.contains(PLAIN_SUBSTITUTE));
    assert!(!flat.contains(PLAIN_PATTERN));
    assert!(editor.state().composition.is_none());
}

#[test]
fn replacement_fires_on_explicit_commit() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::ReplaceSelection {
                text: PLAIN_PATTERN.into(),
            },
            FlatImeOp::CommitAsIs,
        ],
    });

    let flat = flat_text(&editor);
    assert!(flat.contains(PLAIN_SUBSTITUTE));
    assert!(!flat.contains(PLAIN_PATTERN));
}

#[test]
fn update_then_update_keeps_composition_intact() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    let partial = &PLAIN_PATTERN[..PLAIN_PATTERN.len() - 1];
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose {
            text: partial.into(),
        }],
    });
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::Compose {
            text: PLAIN_PATTERN.into(),
        }],
    });

    let flat = flat_text(&editor);
    assert!(flat.contains(PLAIN_PATTERN), "got: {flat:?}");
    assert!(!flat.contains(PLAIN_SUBSTITUTE), "got: {flat:?}");
    assert_eq!(
        editor.state().composition,
        Some(Composition {
            start: 1,
            end: 1 + PLAIN_PATTERN.chars().count(),
        })
    );
}

#[test]
fn plain_rule_applies_on_flat_ime_typing() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    for (i, ch) in PLAIN_PATTERN.chars().enumerate() {
        let cursor = 1 + i;
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection {
                    start: cursor,
                    end: cursor,
                },
                FlatImeOp::ReplaceSelection {
                    text: ch.to_string(),
                },
            ],
        });
    }

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("X") } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn deletion_via_flat_ime_does_not_fire_replacement() {
    // Deleting back to a matching tail must not replace — mirrors the key-path
    // semantics where only insertions trigger, and keeps backspace-restore stable.
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("abcz") } } }
        selection: (p1, 4)
    };
    let mut editor = editor_with_rules(s, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::DeleteSurrounding {
            before: 1,
            after: 0,
        }],
    });

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("abc") } } }
        selection: (p1, 3)
    };
    assert_state_eq!(editor.state(), &expected);
}

#[test]
fn auto_replacement_preserves_bold_of_replaced_char() {
    let (s, p1) = state! {
        doc { root { p1: paragraph {} } }
        selection: (p1, 0)
        pending_modifiers: [bold]
    };
    let mut editor = editor_with_rules(s, vec![rule("\"", "\u{201C}", false)]);

    type_text(&mut editor, "\"");

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("\u{201C}") [bold] } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
    let _ = p1;
}

#[test]
fn nonempty_auto_replacement_writes_no_carry() {
    let (s, p1) = state! {
        doc { root { p1: paragraph { text("a") [bold] } } }
        selection: (p1, 1)
        pending_modifiers: [bold]
    };
    let mut editor = editor_with_rules(s, vec![rule("ab", "X", false)]);

    type_text(&mut editor, "b");

    let (expected, ..) = state! {
        doc { root { p1: paragraph { text("X") [bold] } } }
        selection: (p1, 1)
    };
    assert_state_eq!(editor.state(), &expected);
    assert!(
        editor.state().projected.carry_modifiers(p1).is_empty(),
        "a non-empty substitute must not write carry, got {:?}",
        editor.state().projected.carry_modifiers(p1)
    );
}

#[test]
fn empty_auto_replacement_that_empties_block_updates_carry() {
    let (s, p1) = state! {
        doc { root { p1: paragraph { text("a") [bold] } } }
        selection: (p1, 1)
        pending_modifiers: [bold]
    };
    let mut editor = editor_with_rules(s, vec![rule("ab", "", false)]);

    type_text(&mut editor, "b");

    let flat = flat_text(&editor);
    assert!(
        !flat.contains('a') && !flat.contains('b'),
        "block must be emptied by the empty substitute, got {flat:?}"
    );
    let carry = editor.state().projected.carry_modifiers(p1);
    assert!(
        carry.values().any(|m| matches!(m, Modifier::Bold)),
        "emptying the block via an empty substitute must carry the deleted paint, got {carry:?}"
    );
}

#[test]
fn auto_replacement_undo_restores_matched_text_in_one_step() {
    let (s, p1) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule("abc", "X", false)]);

    for ch in "abc".chars() {
        type_text(&mut editor, &ch.to_string());
    }
    let after = flat_text(&editor);
    assert!(
        after.contains('X') && !after.contains('a'),
        "replacement produced {after:?}"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    let restored = flat_text(&editor);
    assert!(
        restored.contains("abc") && !restored.contains('X'),
        "one undo of the atomic replacement restores the matched text, got {restored:?}"
    );
    let _ = p1;
}

#[test]
fn lookbehind_sees_text_beyond_the_start_window() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(
        s,
        vec![
            rule("(?<!\u{201C}[^\u{201D}]*)\"", "\u{201C}", true),
            rule("(?<=\u{201C}[^\u{201D}]*)\"", "\u{201D}", true),
        ],
    );

    let body = "가".repeat(100);
    type_text(&mut editor, "\"");
    type_text(&mut editor, &body);
    type_text(&mut editor, "\"");

    let text = flat_text(&editor);
    assert!(
        text.contains(&format!("\u{201C}{body}\u{201D}")),
        "quote longer than the start window must still close, got {text:?}"
    );
}

#[test]
fn linear_rule_matches_longer_than_the_start_window() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = editor_with_rules(s, vec![rule(REGEX_PATTERN, REGEX_SUBSTITUTE, true)]);

    type_text(&mut editor, &"7".repeat(300));
    type_text(&mut editor, "#");

    let text = flat_text(&editor);
    assert!(
        text.contains(REGEX_SUBSTITUTE) && !text.contains('7'),
        "the whole match must be replaced, not just its tail, got {text:?}"
    );
}
