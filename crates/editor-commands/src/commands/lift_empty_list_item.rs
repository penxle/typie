use editor_model::NodeType;
use editor_transaction::Transaction;

use crate::helpers::lift_list_item_inner;
use crate::{CommandError, CommandResult};

pub fn lift_empty_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let list_item_id = {
        let view = tr.view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        if node.first_child().is_some() {
            return Ok(false);
        }

        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        if parent.node_type() != NodeType::ListItem {
            return Ok(false);
        }
        parent.id()
    };

    lift_list_item_inner(tr, list_item_id)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn lift_empty_top_level() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph {} }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_empty_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p1: paragraph {}
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph {} }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 0)
        };
        transact_fail!(initial, |tr| lift_empty_list_item(&mut tr));
    }

    #[test]
    fn non_empty_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_empty_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_empty_list_item(&mut tr));
    }

    #[test]
    fn lift_empty_nested() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("outer") }
                            bullet_list {
                                list_item { p1: paragraph {} }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_empty_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("outer") } }
                        list_item { p1: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
