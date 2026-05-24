use editor_macros::state;

use crate::editor::Editor;
use crate::message::*;
use crate::test_utils::{assert_probe_is_safe, assert_probe_predicts_apply};

fn fixture_state() -> editor_state::State {
    let (s, ..) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 2)
    };
    s
}

fn build_corpus() -> Vec<Message> {
    vec![
        Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        },
        Message::Deletion {
            op: DeletionOp::Selection,
        },
        Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: editor_model::ModifierType::Bold,
            },
        },
        Message::Modifier {
            op: ModifierOp::ClearAll,
        },
        Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::All,
            },
        },
        Message::Selection {
            op: SelectionOp::Unset,
        },
        Message::Navigation {
            op: NavigationOp::Move {
                movement: editor_common::Movement::Grapheme {
                    direction: editor_common::Direction::Forward,
                },
                extend: false,
            },
        },
        Message::Navigation {
            op: NavigationOp::Move {
                movement: editor_common::Movement::Line {
                    direction: editor_common::Direction::Forward,
                    axis: editor_common::Axis::Vertical,
                },
                extend: false,
            },
        },
        Message::History {
            op: HistoryOp::Undo,
        },
        Message::History {
            op: HistoryOp::Redo,
        },
        Message::Clipboard {
            op: ClipboardOp::Cut,
        },
        Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: String::new(),
            },
        },
        Message::Composition {
            op: CompositionOp::Commit {
                text: String::new(),
            },
        },
        Message::Composition {
            op: CompositionOp::Cancel,
        },
        Message::View {
            op: ViewOp::ToggleFold {
                id: editor_model::NodeId::ROOT,
            },
        },
        Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        },
        Message::System {
            event: SystemEvent::SetFocused { focused: true },
        },
        Message::System {
            event: SystemEvent::SetThemeVariant {
                variant: editor_renderer::ThemeVariant::DarkBlack,
            },
        },
        Message::System {
            event: SystemEvent::Resize {
                width: 1024.0,
                height: 768.0,
                scale_factor: 1.0,
            },
        },
        Message::System {
            event: SystemEvent::Initialize,
        },
        Message::System {
            event: SystemEvent::FontsChanged,
        },
    ]
}

// SystemEvent variants that process fonts/layout synchronously are excluded from
// prediction accuracy checks because their observable effect depends on loaded
// font state that the test fixture does not initialize.
fn build_prediction_corpus() -> Vec<Message> {
    build_corpus()
        .into_iter()
        .filter(|msg| {
            !matches!(
                msg,
                Message::System {
                    event: SystemEvent::Initialize
                        | SystemEvent::FontsChanged
                        | SystemEvent::FontBaseLoaded { .. }
                        | SystemEvent::FontChunkLoaded { .. },
                }
            )
        })
        .collect()
}

#[test]
fn probe_safety_corpus() {
    for msg in build_corpus() {
        assert_probe_is_safe(fixture_state(), msg);
    }
}

#[test]
fn prediction_accuracy_corpus() {
    for msg in build_prediction_corpus() {
        assert_probe_predicts_apply(fixture_state(), msg);
    }
}

// Exhaustive match forces a compile error when a new Message variant is added
// without updating the corpus above.
#[test]
fn message_variants_are_enumerated() {
    let msg = Message::Insertion {
        op: InsertionOp::Text { text: "x".into() },
    };
    match msg {
        Message::Key { .. }
        | Message::Pointer { .. }
        | Message::Insertion { .. }
        | Message::Deletion { .. }
        | Message::Selection { .. }
        | Message::Modifier { .. }
        | Message::Node { .. }
        | Message::View { .. }
        | Message::Clipboard { .. }
        | Message::Composition { .. }
        | Message::Navigation { .. }
        | Message::History { .. }
        | Message::System { .. }
        | Message::Remote { .. } => {}
    }
}

#[test]
fn can_does_not_leave_editor_in_probe_after_normal_use() {
    let (state, ..) = state! {
        doc { root { paragraph { t1: text("hi") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(state);
    let _ = editor
        .can(Message::Selection {
            op: SelectionOp::Unset,
        })
        .unwrap();
    // can() is read-only; selection is still set after the first call.
    // The second probe must also see a potential change (Unset on a live selection).
    let probed = editor
        .can(Message::Selection {
            op: SelectionOp::Unset,
        })
        .unwrap();
    assert!(
        probed,
        "second probe must still predict a change — can() must not mutate state"
    );
}

#[test]
fn probe_guard_drop_restores_mode_on_panic() {
    use std::panic::AssertUnwindSafe;

    let (state, ..) = state! {
        doc { root { paragraph { t1: text("hi") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(state);

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = crate::editor::probe_guard_for_test(&mut editor);
        panic!("simulated");
    }));
    assert!(result.is_err());
    // ProbeGuard::drop must restore Apply mode so the next can() works normally.
    let probed = editor
        .can(Message::Selection {
            op: SelectionOp::Unset,
        })
        .unwrap();
    assert!(probed, "can() must work after panic-drop of ProbeGuard");
}
