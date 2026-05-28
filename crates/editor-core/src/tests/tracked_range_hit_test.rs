use editor_macros::state;
use editor_state::Selection;

use crate::editor::Editor;
use crate::message::*;

fn add_msg(id: &str, group: &str, sel: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: group.into(),
            selection: sel,
            metadata: String::new(),
        },
    }
}

fn make_test_editor(state: editor_state::State) -> Editor {
    let mut editor = Editor::new_test(state);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor
}

#[test]
fn hit_test_returns_single_range_when_only_one_covers_point() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("a", "comment", sel));

    let range = editor.tracked_ranges().get("a").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, None);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "a");
}

#[test]
fn hit_test_returns_empty_for_point_outside_any_range() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("a", "comment", sel));
    let hits = editor.tracked_ranges_at(0, -100.0, -100.0, None);
    assert!(hits.is_empty());
}

#[test]
fn hit_test_excludes_invalid_range() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("a", "comment", sel));
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Invalidate { id: "a".into() },
    });

    let range = editor.tracked_ranges().get("a").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, None);
    assert!(hits.is_empty(), "invalid range must not appear in hits");
}

#[test]
fn hit_test_filters_by_group() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("a", "comment", sel));
    editor.apply(add_msg("b", "spellcheck", sel));

    let range = editor.tracked_ranges().get("a").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let comment_hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, Some("comment"));
    assert_eq!(comment_hits.len(), 1);
    assert_eq!(comment_hits[0].id, "a");

    let all_hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, None);
    assert_eq!(all_hits.len(), 2);
}

#[test]
fn hit_test_sorts_overlapping_ranges_narrowest_first() {
    let (state, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 11)
    };
    let outer = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("outer", "comment", outer));

    let inner = Selection::new(
        editor_state::Position::new(t1, 0),
        editor_state::Position::new(t1, 5),
    );
    editor.apply(add_msg("inner", "comment", inner));

    let range = editor.tracked_ranges().get("inner").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, None);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id, "inner", "narrower (inner) must come first");
    assert_eq!(hits[1].id, "outer");
}

#[test]
fn hit_test_ignores_decoration_enabled_flag() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    editor.apply(add_msg("a", "comment", sel));
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::SetGroupDecoration {
            group: "comment".into(),
            style: editor_common::DecorationStyle {
                background: Some("selection".into()),
                underline: None,
                ..Default::default()
            },
            enabled: false,
            z_index: 0,
        },
    });

    let range = editor.tracked_ranges().get("a").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, Some("comment"));
    assert_eq!(
        hits.len(),
        1,
        "disabled decoration must not hide range from hit-test"
    );
}

#[test]
fn hit_test_breaks_char_count_ties_by_id_alphabetically() {
    // 같은 문자 길이의 두 range가 같은 좌표에 있을 때 id 사전순으로 정렬되는지 검증.
    let (state, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 0) -> (t1, 5)
    };
    let sel = state.selection.unwrap();
    let mut editor = make_test_editor(state);
    // 같은 selection으로 두 range를 등록 (서로 다른 id로).
    editor.apply(add_msg("zzz", "comment", sel));
    editor.apply(add_msg("aaa", "comment", sel));

    let range = editor.tracked_ranges().get("aaa").unwrap();
    let resolved = range
        .selection
        .thaw(&editor.state().doc)
        .resolve(&editor.state().doc)
        .unwrap();
    let rects = editor.view().selection_rects(&resolved);
    let r = rects[0].rect;
    let cx = r.x + r.width * 0.5;
    let cy = r.y + r.height * 0.5;

    let hits = editor.tracked_ranges_at(rects[0].page_idx, cx, cy, None);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id, "aaa", "ties broken by id alphabetical order");
    assert_eq!(hits[1].id, "zzz");
}
