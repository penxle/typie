use editor_model::Node;
use editor_transaction::Transaction;

use crate::helpers::{find_enclosing_list_item_id, lift_list_item_inner};
use crate::{CommandError, CommandResult};

pub fn lift_first_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let list_item_id = match find_enclosing_list_item_id(&doc, pos.node_id) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = doc
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = list_item.first_child().ok_or(CommandError::Corrupted(
        "list_item missing paragraph".into(),
    ))?;
    let paragraph_id = paragraph.id();

    // Only fire when the cursor is anchored at the very start of the list_item's
    // paragraph — either at offset 0 of the paragraph itself, or at offset 0 of
    // its first inline child.
    match node.node() {
        Node::Text(_) => {
            if node.prev_sibling().is_some() {
                return Ok(false);
            }
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            if parent.id() != paragraph_id {
                return Ok(false);
            }
        }
        Node::Paragraph(_) => {
            if node.id() != paragraph_id {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }

    if list_item.prev_sibling().is_some() {
        return Ok(false);
    }

    lift_list_item_inner(tr, list_item_id)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn lift_first_item_top_level() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn prev_item_exists_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }
}
