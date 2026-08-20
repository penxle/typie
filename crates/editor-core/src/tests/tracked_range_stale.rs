use editor_common::{Direction, Movement};
use editor_macros::state;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::event::EditorEvent;
use crate::message::*;

fn stale_ids(events: &[EditorEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|ev| match ev {
            EditorEvent::TrackedRangesStale { ids } => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn add_sensitive(id: &str, sel: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: "spellcheck".into(),
            selection: sel,
            metadata: String::new(),
            invalidate_on_text_change: true,
        },
    }
}

fn set_cursor(editor: &mut Editor, node: editor_crdt::Dot, offset: usize) {
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(node, offset)),
        },
    });
}

fn hello_world_editor() -> (Editor, editor_crdt::Dot) {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0) -> (p1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_sensitive("r1", sel));
    (editor, p1)
}

#[test]
fn install_captures_covered_text() {
    let (editor, _p1) = hello_world_editor();
    let range = editor.tracked_ranges().get("r1").expect("installed");
    assert_eq!(range.captured_text.as_deref(), Some("hello"));
    assert!(range.invalidate_on_text_change);
    assert!(!range.covered_blocks.is_empty());
}

#[test]
fn backspace_inside_sensitive_range_reports_stale() {
    let (mut editor, p1) = hello_world_editor();
    set_cursor(&mut editor, p1, 3);
    let events = editor.apply(Message::Deletion {
        op: DeletionOp::Surrounding {
            before: 1,
            after: 0,
        },
    });
    assert!(!editor.tracked_ranges().contains("r1"));
    assert_eq!(stale_ids(&events), vec!["r1".to_string()]);
}

#[test]
fn deleting_covering_selection_reports_stale() {
    let (mut editor, p1) = hello_world_editor();
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(Position::new(p1, 0), Position::new(p1, 11)),
        },
    });
    let events = editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert!(!editor.tracked_ranges().contains("r1"));
    assert_eq!(stale_ids(&events), vec!["r1".to_string()]);
}

#[test]
fn typing_at_range_boundaries_keeps_it() {
    let (mut editor, p1) = hello_world_editor();

    set_cursor(&mut editor, p1, 5);
    let after_end = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert!(
        editor.tracked_ranges().contains("r1"),
        "typing immediately after the range must not stale it"
    );
    assert!(stale_ids(&after_end).is_empty());

    set_cursor(&mut editor, p1, 0);
    let before_start = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "Y".into() },
    });
    assert!(
        editor.tracked_ranges().contains("r1"),
        "typing immediately before the range must not stale it"
    );
    assert!(stale_ids(&before_start).is_empty());
}

#[test]
fn group_transitions_do_not_change_text_staleness() {
    for groups in [
        &[][..],
        &["active"][..],
        &["active", "hover", "selected"][..],
    ] {
        let (mut editor, p1) = hello_world_editor();
        for group in groups {
            editor.apply(Message::TrackedRange {
                op: TrackedRangeOp::SetGroup {
                    id: "r1".into(),
                    group: (*group).into(),
                },
            });
        }

        set_cursor(&mut editor, p1, 2);
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });

        assert!(!editor.tracked_ranges().contains("r1"));
        assert_eq!(
            stale_ids(&events),
            vec!["r1".to_string()],
            "0/1/multiple group transitions must preserve staleness (groups={groups:?})"
        );
    }
}

#[test]
fn paragraph_split_outside_range_keeps_it() {
    let (mut editor, p1) = hello_world_editor();
    set_cursor(&mut editor, p1, 8);
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Break {
            kind: Break::Paragraph,
        },
    });
    assert!(
        editor.tracked_ranges().contains("r1"),
        "splitting the paragraph outside the range must keep it"
    );
    assert!(stale_ids(&events).is_empty());
}

#[test]
fn paragraph_split_inside_range_keeps_it_while_text_is_intact() {
    // A break inserts no characters, so the covered text still reads
    // "hello" across the two halves — the same verdict the legacy
    // host-side `r.text !== context` compare produced. Deleting a covered
    // character afterwards must still stale it from its new two-block home.
    let (mut editor, p1) = hello_world_editor();
    set_cursor(&mut editor, p1, 2);
    let split = editor.apply(Message::Insertion {
        op: InsertionOp::Break {
            kind: Break::Paragraph,
        },
    });
    assert!(editor.tracked_ranges().contains("r1"));
    assert!(stale_ids(&split).is_empty());

    let events = editor.apply(Message::Deletion {
        op: DeletionOp::Surrounding {
            before: 0,
            after: 1,
        },
    });
    assert!(!editor.tracked_ranges().contains("r1"));
    assert_eq!(stale_ids(&events), vec!["r1".to_string()]);
}

#[test]
fn undo_does_not_resurrect_stale_range() {
    let (mut editor, p1) = hello_world_editor();
    set_cursor(&mut editor, p1, 2);
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert!(!editor.tracked_ranges().contains("r1"));

    let events = editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert!(
        !editor.tracked_ranges().contains("r1"),
        "undo restores text but never resurrects a removed range"
    );
    assert!(stale_ids(&events).is_empty());
}

#[test]
fn transient_text_change_before_navigation_is_verified_at_tick_end() {
    let (mut editor, p1) = hello_world_editor();
    set_cursor(&mut editor, p1, 2);

    let request_id = editor
        .enqueue_request(vec![
            Message::Insertion {
                op: InsertionOp::Text { text: "X".into() },
            },
            Message::Navigation {
                op: NavigationOp::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                    extend: false,
                },
            },
            Message::History {
                op: HistoryOp::Undo,
            },
        ])
        .unwrap();
    let events = editor.tick_through(request_id).unwrap().events;

    assert!(
        editor.tracked_ranges().contains("r1"),
        "a range whose captured text matches the final state must survive the tick"
    );
    assert!(stale_ids(&events).is_empty());
}

#[test]
fn prose_installed_sensitive_ranges_report_stale() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::ReplaceGroupsFromProse {
            expected_text: "hello world".into(),
            groups: vec!["spellcheck".into()],
            ranges: vec![ProseTrackedRangeRegistration {
                id: "e1".into(),
                group: "spellcheck".into(),
                start: 6,
                end: 11,
                metadata: String::new(),
                invalidate_on_text_change: true,
            }],
        },
    });
    assert_eq!(
        editor
            .tracked_ranges()
            .get("e1")
            .and_then(|r| r.captured_text.as_deref()),
        Some("world")
    );

    set_cursor(&mut editor, p1, 8);
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert!(!editor.tracked_ranges().contains("e1"));
    assert_eq!(stale_ids(&events), vec!["e1".to_string()]);
}

#[test]
fn only_ranges_in_edited_block_are_stale_checked() {
    let (initial, _p1, p2) = state! {
        doc { root {
            p1: paragraph { text("hello world" ) }
            p2: paragraph { text("lorem ipsum") }
        } }
        selection: (p1, 0) -> (p1, 5)
    };
    let sel1 = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_sensitive("r1", sel1));
    editor.apply(add_sensitive(
        "r2",
        Selection::new(Position::new(p2, 0), Position::new(p2, 5)),
    ));

    set_cursor(&mut editor, p2, 2);
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert!(editor.tracked_ranges().contains("r1"));
    assert!(!editor.tracked_ranges().contains("r2"));
    assert_eq!(stale_ids(&events), vec!["r2".to_string()]);
}

#[test]
fn stale_range_no_longer_produces_decoration_marks() {
    let (mut editor, p1) = hello_world_editor();
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::SetGroupDecoration {
            group: "spellcheck".into(),
            style: editor_common::DecorationStyle {
                background: Some("selection".into()),
                underline: None,
                ..Default::default()
            },
            enabled: true,
            z_index: 0,
        },
    });
    assert!(!editor.tracked_decoration_marks_for_test().is_empty());

    set_cursor(&mut editor, p1, 2);
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert!(
        editor.tracked_decoration_marks_for_test().is_empty(),
        "stale removal must drop the range's decoration in the same tick"
    );
}

#[test]
fn decoration_marks_cache_follows_reflow() {
    let (mut editor, p1) = hello_world_editor();
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::SetGroupDecoration {
            group: "spellcheck".into(),
            style: editor_common::DecorationStyle {
                background: Some("selection".into()),
                underline: None,
                ..Default::default()
            },
            enabled: true,
            z_index: 0,
        },
    });

    let first_x = |editor: &Editor| -> f32 {
        let marks = editor.tracked_decoration_marks_for_test();
        marks
            .first()
            .and_then(|m| m.rects.first())
            .map(|r| r.rect.x)
            .expect("decoration mark rect")
    };

    let before = first_x(&editor);
    assert_eq!(
        before,
        first_x(&editor),
        "repeated reads within one render epoch return identical marks"
    );

    set_cursor(&mut editor, p1, 0);
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "YY".into() },
    });

    assert!(
        editor.tracked_ranges().contains("r1"),
        "typing before the range keeps it"
    );
    assert!(
        first_x(&editor) > before,
        "a doc edit that shifts the range must refresh the cached decoration marks"
    );
}
