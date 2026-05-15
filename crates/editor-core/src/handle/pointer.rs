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
            let raw_hit = editor.view.hit_test(page, x, y);
            let ext_hit = editor.view.hit_test_extending(page, x, y);

            let selection = match count {
                0 => return Ok(()),
                1 => {
                    if modifiers.shift {
                        ext_hit
                            .as_ref()
                            .map(|h| Selection::new(editor.state.selection.anchor, h.head))
                    } else {
                        raw_hit
                    }
                }
                2 => {
                    let resolved = raw_hit
                        .as_ref()
                        .and_then(|s| s.head.resolve(&editor.state.doc));
                    let resource = editor.resource.lock().unwrap();
                    resolved
                        .and_then(|rp| editor.view.select_word_at(&rp, &resource))
                        .or(raw_hit)
                }
                3.. => {
                    let pos = raw_hit.as_ref().map(|s| &s.head);
                    pos.and_then(|p| editor.view.select_paragraph_at(p))
                        .or(raw_hit)
                }
            };

            if let Some(new_selection) = selection {
                editor.view.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }

            // Drag anchor is promotion-aware: a drag started in the gutter near
            // a monolithic block must anchor at the block boundary, not at the
            // nearest leaf inside it — otherwise the promoted Move forms an
            // adjacent slot/descendant range that normalize collapses. Plain
            // (non-promoting) positions make ext_hit == raw_hit, so the anchor
            // is unchanged where promotion does not apply.
            editor.drag_anchor = (count == 1).then(|| {
                if modifiers.shift {
                    editor.state.selection.anchor
                } else {
                    ext_hit
                        .as_ref()
                        .map(|h| h.head)
                        .unwrap_or(editor.state.selection.anchor)
                }
            });
        }

        PointerEvent::Move { page, x, y } => {
            let Some(anchor) = editor.drag_anchor else {
                return Ok(());
            };

            if let Some(hit) = editor.view.hit_test_extending(page, x, y) {
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

    #[test]
    fn drag_envelopes_leading_fold_without_anchor_collapse() {
        let (state, ta) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "drag anchor must not jump to another node"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert_eq!(
            sel.anchor.node_id, ta,
            "anchor must remain in the trailing paragraph text"
        );
        assert!(
            !sel.is_collapsed(),
            "selection enveloping the leading fold must survive normalize, got {:?}",
            sel
        );
    }

    #[test]
    fn plain_gutter_down_does_not_promote_to_block() {
        let (state, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: -9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let sel = editor.state().selection;
        assert!(sel.is_collapsed());
        assert_ne!(
            sel.head.node_id,
            editor_model::NodeId::ROOT,
            "plain gutter Down must not promote to a container slot"
        );
    }

    #[test]
    fn drag_started_in_gutter_above_leading_fold_envelopes_it() {
        let (state, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: -9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            editor.drag_anchor.is_some(),
            "Down must arm a drag anchor (count==1)"
        );

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection;
        assert!(
            !sel.is_collapsed(),
            "gutter-started drag must envelope the leading fold, not collapse, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor.node_id,
            editor_model::NodeId::ROOT,
            "drag_anchor must use ext_hit (promoted block boundary = ROOT slot 0), \
             not raw_hit (a leaf inside fold_title): this assertion is the load-bearing \
             discriminator for the promotion-aware drag_anchor — !is_collapsed() alone \
             would still pass under a raw_hit regression"
        );
        assert_eq!(
            sel.anchor.offset, 0,
            "promoted Front slot of the leading fold is offset 0 under ROOT"
        );
    }

    #[test]
    fn drag_below_envelopes_fold_with_trailing_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { ta: text("before") }
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (ta, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "anchor must stay in the leading paragraph (affinity may flip)"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert!(
            !sel.is_collapsed(),
            "drag below a fold (with trailing paragraph) must span it, got {:?}",
            sel
        );
    }

    #[test]
    fn drag_up_past_fold_with_textless_neighbor_envelopes_it() {
        let (state, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after") }
                }
            }
            selection: (ta, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "anchor must stay in the trailing paragraph (affinity may flip)"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert!(
            !sel.is_collapsed(),
            "drag up past a fold with a text-less neighbor must span it, got {:?}",
            sel
        );
    }
}
