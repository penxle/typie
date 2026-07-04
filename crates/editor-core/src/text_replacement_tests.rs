use editor_macros::state;
use editor_model::Modifier;
use editor_resource::RawTextReplacementRule;
use editor_state::{
    Composition, PendingModifier, PendingStyle, Position, ResolvedPosition,
    ResolvedPositionFlatExt, Selection, assert_state_eq,
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

fn set_rules(editor: &Editor, rules: Vec<RawTextReplacementRule>) {
    editor
        .resource
        .lock()
        .unwrap()
        .set_text_replacement_rules(rules);
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

const PLAIN_PATTERN: &str = "abc";
const PLAIN_SUBSTITUTE: &str = "X";
const REGEX_PATTERN: &str = r"\d+#";
const REGEX_SUBSTITUTE: &str = "N";
const MULTILINE_SUB_PATTERN: &str = "mk";
const MULTILINE_SUB_SUBSTITUTE: &str = "a\nb";
const MULTILINE_PAT_PATTERN: &str = r"P1\nP2";
const MULTILINE_PAT_SUBSTITUTE: &str = "Q";

#[test]
fn plain_rule_applies_on_insertion() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(REGEX_PATTERN, REGEX_SUBSTITUTE, true)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(
        &editor,
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
    let mut editor = Editor::new_test(s);
    set_rules(
        &editor,
        vec![rule(MULTILINE_PAT_PATTERN, MULTILINE_PAT_SUBSTITUTE, true)],
    );

    type_text(&mut editor, "2");

    let flat = flat_text(&editor);
    assert!(flat.contains(MULTILINE_PAT_SUBSTITUTE), "got: {flat:?}");
    assert!(!flat.contains("P1"), "got: {flat:?}");
    assert!(!flat.contains("P2"), "got: {flat:?}");
}

#[test]
fn backspace_immediately_after_replacement_restores_original() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
        doc {
            styles { s1: "s" [] }
            root { p1: paragraph { text("") } }
        }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);
    editor
        .transact(|tr| {
            tr.set_pending_modifiers(vec![PendingModifier::Set {
                modifier: Modifier::Bold,
            }])?;
            tr.set_pending_style(Some(PendingStyle::Set {
                style_id: "s1".to_string(),
            }))?;
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
    assert!(
        editor.state().pending_style.is_none(),
        "pending style cleared"
    );
    assert!(editor.undo_history.can_redo(), "redo stack must be intact");
}

#[test]
fn second_backspace_after_restore_is_normal_delete() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
fn typing_after_replacement_invalidates_restore() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(
        &editor,
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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
fn flat_text_input_message_commits_preedit() {
    let (s, ..) = state! {
        doc { root { p1: paragraph { text("") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
    let mut editor = Editor::new_test(s);
    set_rules(&editor, vec![rule(PLAIN_PATTERN, PLAIN_SUBSTITUTE, false)]);

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
