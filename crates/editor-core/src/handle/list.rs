use editor_commands::{self as commands};
use editor_model::NodeType;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_list_op(editor: &mut Editor, op: ListOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match op {
            ListOp::ToggleKind { kind } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::materialize_synthetic_selection_blocks()),
                    |tr| commands::first!(
                        tr,
                        commands::lift_list_items_of_kind(list_kind_to_node_type(kind)),
                        commands::set_list_kind(list_kind_to_node_type(kind)),
                    ),
                )?;
            }
            ListOp::Indent => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_synthetic_selection_blocks()),
                    commands::sink_list_item(),
                )?;
            }
            ListOp::Outdent => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_synthetic_selection_blocks()),
                    commands::lift_list_item(),
                )?;
            }
        }
        Ok(())
    })
}

fn list_kind_to_node_type(kind: ListKind) -> NodeType {
    match kind {
        ListKind::Bullet => NodeType::BulletList,
        ListKind::Ordered => NodeType::OrderedList,
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_state::{Affinity, Position, Selection, assert_state_eq};

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn list_message(op: ListOp) -> Message {
        Message::List { op }
    }

    #[test]
    fn toggle_kind_converts_ordered_list_to_bullet() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(list_message(ListOp::ToggleKind {
            kind: ListKind::Bullet,
        }));

        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn indent_uses_list_item_command_for_range_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(list_message(ListOp::Indent));

        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn outdent_uses_list_item_command_for_range_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(list_message(ListOp::Outdent));

        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn probe_predicts_toggle_kind_apply() {
        let (state, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(
            state,
            list_message(ListOp::ToggleKind {
                kind: ListKind::Bullet,
            }),
        );
    }

    #[test]
    fn can_toggle_kind_for_full_document_range_with_list() {
        let (mut state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    ordered_list { list_item { p2: paragraph { text("B") } } }
                    p3: paragraph { text("C") }
                }
            }
            selection: (p1, 0)
        };
        state.selection = Some(Selection::new(
            Position {
                node: Dot::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: Dot::ROOT,
                offset: 3,
                affinity: Affinity::Upstream,
            },
        ));
        let mut editor = Editor::new_test(state);

        assert!(
            editor
                .can(list_message(ListOp::ToggleKind {
                    kind: ListKind::Bullet,
                }))
                .unwrap()
        );
    }

    #[test]
    fn toggle_kind_can_apply_to_pure_paragraph_range() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let mut editor = Editor::new_test(state);

        assert!(
            editor
                .can(list_message(ListOp::ToggleKind {
                    kind: ListKind::Bullet,
                }))
                .unwrap()
        );
    }

    #[test]
    fn toggle_kind_lifts_same_kind_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(list_message(ListOp::ToggleKind {
            kind: ListKind::Bullet,
        }));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
