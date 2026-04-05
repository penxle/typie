use editor_commands::{self as commands};
use editor_state::Selection;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_deletion_intent(
    editor: &mut Editor,
    intent: DeletionIntent,
) -> Result<(), EditorError> {
    match intent {
        DeletionIntent::Selection => {
            editor.transact(|tr| {
                commands::delete_selection(tr)?;
                Ok(())
            })?;
        }
        DeletionIntent::Move {
            movement:
                Movement::Grapheme {
                    direction: Direction::Backward,
                },
        } => {
            editor.transact(|tr| {
                commands::delete_text_backward(tr)?;
                Ok(())
            })?;
        }
        DeletionIntent::Move {
            movement:
                Movement::Grapheme {
                    direction: Direction::Forward,
                },
        } => {
            editor.transact(|tr| {
                commands::delete_text_forward(tr)?;
                Ok(())
            })?;
        }
        DeletionIntent::Move { movement } => {
            let head = editor.state().selection.head;
            let segmenters = editor.resource.lock().unwrap().segmenters.clone();
            let target = editor.view.resolve_movement(
                &head,
                &movement,
                &editor.state.doc,
                segmenters.as_deref(),
            );

            if let Some(target) = target {
                let selection = Selection::new(head, target.head);
                editor.transact(|tr| {
                    commands::set_selection(tr, selection)?;
                    commands::delete_selection(tr)?;
                    Ok(())
                })?;
            }
        }
        DeletionIntent::Surrounding {
            before: _,
            after: _,
        } => {}
        DeletionIntent::SurroundingCodePoints {
            before: _,
            after: _,
        } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::Resource;
    use editor_resource::TextSegmenters;
    use editor_state::assert_state_eq;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::editor::Editor;

    #[test]
    fn delete_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 2) -> (t1, 8)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Selection,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("herld") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_grapheme_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    fn editor_with_layout(state: editor_state::State) -> Editor {
        let resource = Arc::new(Mutex::new(Resource::new()));
        resource.lock().unwrap().segmenters = Some(Arc::new(TextSegmenters::new_test()));
        let mut editor = Editor::new_test_with_resource(state.clone(), resource);
        editor.view.layout(&state.doc);
        editor
    }

    #[test]
    fn delete_word_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello ") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Forward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text(" world") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Line {
                        direction: Direction::Backward,
                        axis: editor_common::Axis::Horizontal,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text(" world") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_line_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 5)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Line {
                        direction: Direction::Forward,
                        axis: editor_common::Axis::Horizontal,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_at_doc_start_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = editor_with_layout(state);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_word_without_segmenters_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        editor.apply(Message::Intent {
            intent: Intent::Deletion {
                intent: DeletionIntent::Move {
                    movement: Movement::Word {
                        direction: Direction::Backward,
                    },
                },
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
