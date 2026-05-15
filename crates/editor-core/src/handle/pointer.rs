use editor_state::Selection;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_pointer_event(editor: &mut Editor, event: PointerEvent) -> Result<(), EditorError> {
    match event {
        PointerEvent::Down {
            page,
            x,
            y,
            count,
            modifiers,
        } => {
            let hit = editor.view.hit_test(page, x, y);

            let selection = match count {
                0 => return Ok(()),
                1 => {
                    if modifiers.shift {
                        hit.map(|h| Selection::new(editor.state.selection.anchor, h.head))
                    } else {
                        hit
                    }
                }
                2 => {
                    let resolved = hit.as_ref().and_then(|s| s.head.resolve(&editor.state.doc));
                    let resource = editor.resource.lock().unwrap();
                    resolved
                        .and_then(|rp| editor.view.select_word_at(&rp, &resource))
                        .or(hit)
                }
                3.. => {
                    let pos = hit.as_ref().map(|s| &s.head);
                    pos.and_then(|p| editor.view.select_paragraph_at(p)).or(hit)
                }
            };

            if let Some(new_selection) = selection {
                editor.view.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }

            editor.drag_anchor = (count == 1).then_some(editor.state.selection.anchor);
        }

        PointerEvent::Move { page, x, y } => {
            let Some(anchor) = editor.drag_anchor else {
                return Ok(());
            };

            if let Some(hit) = editor.view.hit_test(page, x, y) {
                let new_selection = Selection::new(anchor, hit.head);
                editor.view.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }
        }

        PointerEvent::Up => {
            editor.drag_anchor = None;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn double_click_fallback_when_no_layout() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 2,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn triple_click_fallback_when_no_layout() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 3,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn high_click_count_treated_as_paragraph() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 5,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn shift_click_extends_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 9999.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        });

        let sel = editor.state().selection;
        assert_eq!(sel.anchor, anchor);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn drag_extends_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(sel.anchor, anchor);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn drag_anchor_survives_intermediate_collapse() {
        use editor_state::Position;

        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let drag_anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.state.selection = Selection::collapsed(Position::new(t, 11));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor, drag_anchor,
            "drag anchor must survive an intermediate collapse"
        );
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn move_without_drag_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 50.0,
                y: 5.0,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(editor.drag_anchor.is_none());
    }

    #[test]
    fn up_resets_dragging() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 0.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });

        assert!(editor.drag_anchor.is_none());
    }
}
