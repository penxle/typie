use editor_common::{Color, Rect};
use editor_macros::state;
use editor_renderer::display_list::PrimPayload;

use crate::{Editor, FlatImeOp, HistoryOp, ImeRange, Message, ModifierOp, SystemEvent, ViewOp};

fn composing_editor() -> Editor {
    let (state, _) = state! {
        doc { root { p: paragraph {} } }
        selection: (p, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(Message::System {
        event: SystemEvent::SetFocused { focused: true },
    });
    editor.apply(Message::TextInput {
        ops: vec![
            FlatImeOp::SetComposition { start: 1, end: 1 },
            FlatImeOp::Compose {
                text: "日本の国".into(),
            },
        ],
    });
    editor
}

fn set_targets(editor: &mut Editor, ranges: Vec<ImeRange>) {
    editor.apply(Message::View {
        op: ViewOp::SetCompositionTargetRanges { ranges },
    });
}

fn backgrounds(editor: &mut Editor) -> Vec<Rect> {
    let color = editor
        .resource()
        .lock()
        .unwrap()
        .theme()
        .color_with_alpha("selection", 77);
    fills(editor, color)
}

fn underlines(editor: &mut Editor) -> Vec<Rect> {
    let color = editor
        .resource()
        .lock()
        .unwrap()
        .theme()
        .color("ui.text.default");
    fills(editor, color)
}

fn fills(editor: &mut Editor, color: Color) -> Vec<Rect> {
    editor
        .build_display_list(0, 1.0)
        .unwrap()
        .0
        .primitives
        .into_iter()
        .filter_map(|p| match p.payload {
            PrimPayload::FillRect {
                rect,
                color: actual,
                ..
            } if actual == color => Some(rect),
            _ => None,
        })
        .collect()
}

#[test]
fn composition_target_moves_the_rendered_background_without_editing_text_or_selection() {
    let mut editor = composing_editor();
    let text = editor.ime(64, 64).unwrap().unwrap();
    let selection = editor.state().selection;
    assert!(backgrounds(&mut editor).is_empty());
    let signature = editor.page_render_signature(0);
    set_targets(&mut editor, vec![ImeRange { start: 1, end: 4 }]);
    assert_ne!(editor.page_render_signature(0), signature);
    let first = backgrounds(&mut editor);
    assert_eq!(first.len(), 1);
    let line_box = editor
        .first_rect_for_range(editor.revision(), 1, 4)
        .unwrap()
        .rect;
    assert_eq!(first[0].x, line_box.x);
    assert_eq!(first[0].width, line_box.width);
    assert!(
        first[0].y > line_box.y,
        "composition background must exclude line spacing"
    );
    assert!(first[0].bottom() < line_box.bottom());
    let underline = underlines(&mut editor);
    assert_eq!(underline.len(), 1);
    assert_eq!(underline[0].height, 1.0);
    assert_eq!(first[0].bottom(), underline[0].bottom());
    set_targets(&mut editor, vec![ImeRange { start: 4, end: 5 }]);
    let last = backgrounds(&mut editor);
    assert_eq!(last.len(), 1);
    assert!(last[0].x > first[0].x);
    assert!(last[0].width < first[0].width);
    assert_eq!(editor.ime(64, 64).unwrap().unwrap(), text);
    assert_eq!(editor.state().selection, selection);
}

#[test]
fn composition_target_does_not_fill_the_whole_composition_or_overlapping_ranges_twice() {
    let mut editor = composing_editor();
    for ranges in [
        vec![ImeRange { start: 1, end: 5 }],
        vec![ImeRange { start: 1, end: 3 }, ImeRange { start: 3, end: 5 }],
        vec![ImeRange { start: 2, end: 2 }],
        vec![ImeRange { start: 5, end: 8 }],
    ] {
        set_targets(&mut editor, ranges);
        assert!(backgrounds(&mut editor).is_empty());
    }
    set_targets(
        &mut editor,
        vec![ImeRange { start: 2, end: 4 }, ImeRange { start: 1, end: 3 }],
    );
    assert_eq!(backgrounds(&mut editor).len(), 1);
}

#[test]
fn composition_target_is_cleared_by_commit_cancel_blur_and_preedit_replacement() {
    for message in [
        Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        },
        Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        },
        Message::System {
            event: SystemEvent::SetFocused { focused: false },
        },
        Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "別の文節".into(),
            }],
        },
        Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 0 },
                FlatImeOp::ReplaceSelection {
                    text: "invalid".into(),
                },
            ],
        },
    ] {
        let mut editor = composing_editor();
        set_targets(&mut editor, vec![ImeRange { start: 4, end: 5 }]);
        assert!(!backgrounds(&mut editor).is_empty());
        editor.apply(message);
        assert!(backgrounds(&mut editor).is_empty());
        editor.apply(Message::System {
            event: SystemEvent::SetFocused { focused: true },
        });
        assert!(backgrounds(&mut editor).is_empty());
    }
}

#[test]
fn composition_target_updates_do_not_create_undo_entries() {
    let mut editor = composing_editor();
    set_targets(&mut editor, vec![ImeRange { start: 1, end: 4 }]);
    set_targets(&mut editor, vec![ImeRange { start: 4, end: 5 }]);
    editor.apply(Message::TextInput {
        ops: vec![FlatImeOp::CommitAsIs],
    });
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(
        editor.ime(64, 64).unwrap().unwrap().text,
        "\u{2028}\u{2029}"
    );
    assert!(backgrounds(&mut editor).is_empty());
}

#[test]
fn composition_target_uses_engine_geometry_across_soft_wraps() {
    let mut previous_metrics = None;
    for line_height in [160, 300] {
        let (state, _) = state! {
            doc { root { p: paragraph { text("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::SetOnNode {
                id: editor.state().selection.unwrap().head.node,
                modifier: editor_model::Modifier::LineHeight { value: line_height },
            },
        });
        editor.apply(Message::System {
            event: SystemEvent::SetFocused { focused: true },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 105 }],
        });
        set_targets(&mut editor, vec![ImeRange { start: 3, end: 103 }]);
        let rects = backgrounds(&mut editor);
        assert!(rects.len() > 1);
        assert!(rects[1].y > rects[0].y);
        let underline = underlines(&mut editor);
        assert_eq!(rects.len(), underline.len());
        for (background, underline) in rects.iter().zip(&underline) {
            assert_eq!(background.bottom(), underline.bottom());
            assert_eq!(underline.height, 1.0);
        }
        let line_spacing = rects[1].y - rects[0].y;
        if let Some((height, spacing)) = previous_metrics {
            assert!(
                line_spacing > spacing,
                "fixture must increase the line spacing"
            );
            assert_eq!(
                rects[0].height, height,
                "line spacing must not enlarge composition decorations"
            );
        }
        previous_metrics = Some((rects[0].height, line_spacing));
    }
}
