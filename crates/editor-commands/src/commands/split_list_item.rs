use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, NodeType, Subtree};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{carryable_modifiers_at, find_enclosing_list_item_id};
use crate::{CommandError, CommandResult};

pub fn split_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let view = tr.view();

    let list_item_id = match find_enclosing_list_item_id(&view, pos.node) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = view
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = match list_item.first_child() {
        Some(ChildView::Block(p)) => p,
        _ => {
            return Err(CommandError::Corrupted(
                "list_item missing paragraph".into(),
            ));
        }
    };
    let paragraph_id = paragraph.id();

    if pos.node != paragraph_id {
        return Ok(false);
    }
    if paragraph.children().count() == 0 {
        return Ok(false);
    }

    let split_index = pos.offset;
    let para_len = paragraph.children().count();

    // The leaves after the cursor cannot be reparented (move/split re-emit drops
    // identity, and a list_item rejects a second paragraph), so capture their
    // char + explicit (non-style) span modifiers and re-create them in the new
    // item with the formatting intact.
    let tail: Vec<(char, Vec<Modifier>)> = paragraph
        .children()
        .skip(split_index)
        .filter_map(|c| match c {
            ChildView::Leaf(l) => l.as_char().map(|ch| {
                let mods = l
                    .own_modifiers()
                    .iter()
                    .filter(|(_, o)| !o.from_style)
                    .map(|(_, o)| o.value.clone())
                    .collect();
                (ch, mods)
            }),
            ChildView::Block(_) => None,
        })
        .collect();
    let tail_len = para_len - split_index;

    let sublist_id: Option<Dot> = list_item
        .child_blocks()
        .find(|b| matches!(b.node_type(), NodeType::BulletList | NodeType::OrderedList))
        .map(|b| b.id());

    let (list_id, li_block_index) = {
        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        let idx = list
            .child_blocks()
            .position(|b| b.id() == list_item_id)
            .ok_or_else(|| CommandError::orphan_child(list_item_id, list.id()))?;
        (list.id(), idx)
    };

    let carryable = carryable_modifiers_at(&view, pos, tr.pending_modifiers());
    drop(view);

    if tail_len > 0 {
        tr.remove_text(paragraph_id, split_index, tail_len)?;
    }

    let new_li = Subtree::leaf(NodeType::ListItem.into_node().to_plain()).with_children(vec![
        Subtree::leaf(NodeType::Paragraph.into_node().to_plain()),
    ]);
    tr.insert_subtree(list_id, li_block_index + 1, new_li)?;

    let (new_list_item_id, new_paragraph_id) = {
        let view = tr.view();
        let list = view
            .node(list_id)
            .ok_or(CommandError::NodeNotFound(list_id))?;
        let new_li = list
            .child_blocks()
            .nth(li_block_index + 1)
            .ok_or(CommandError::Corrupted("new list_item missing".into()))?;
        let new_li_id = new_li.id();
        let new_para = match new_li.first_child() {
            Some(ChildView::Block(p)) => p.id(),
            _ => {
                return Err(CommandError::Corrupted(
                    "new list_item missing paragraph".into(),
                ));
            }
        };
        (new_li_id, new_para)
    };

    if let Some(sublist) = &sublist_id {
        let at = {
            let view = tr.view();
            view.node(new_list_item_id)
                .map(|li| li.child_blocks().count())
                .unwrap_or(1)
        };
        tr.move_node(*sublist, new_list_item_id, at)?;
    }

    if !tail.is_empty() {
        let text: String = tail.iter().map(|(ch, _)| *ch).collect();
        tr.insert_text(new_paragraph_id, 0, &text)?;
        let char_dots: Vec<_> = {
            let view = tr.view();
            view.node(new_paragraph_id)
                .map(|p| {
                    p.children()
                        .filter_map(|c| match c {
                            ChildView::Leaf(l) => l.as_char().map(|_| l.dot()),
                            ChildView::Block(_) => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        for (dot, (_, mods)) in char_dots.iter().zip(tail.iter()) {
            for m in mods {
                tr.add_span_modifier(*dot, *dot, m.clone())?;
            }
        }
    }

    let marker = editor_model::Marker {
        modifiers: carryable,
        style: None,
    };
    if !marker.is_empty() {
        tr.set_marker(new_paragraph_id, Some(marker))?;
    }

    tr.set_selection(Some(Selection::collapsed(Position::new(
        new_paragraph_id,
        0,
    ))))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn split_text_end() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, _p1) = state! {
            doc { root { bullet_list { list_item { p1: paragraph { text("A") } } } paragraph {} } }
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn empty_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root { bullet_list { list_item { p1: paragraph {} } } paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn split_text_middle() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("He") } }
                        list_item { p2: paragraph { text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_text_start() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph {} }
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_with_sublist_moves_sublist_to_new_item() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("Hello") }
                            bullet_list { list_item { paragraph { text("sub") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item {
                            p2: paragraph {}
                            bullet_list { list_item { paragraph { text("sub") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_in_ordered_list() {
        let (initial, _p1) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { p1: paragraph { text("He") } }
                        list_item { p2: paragraph { text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_multiple_text_children() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph {
                                text("Hello")
                                text("World")
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_at_end_attaches_marker_to_new_paragraph() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                        list_item { p2: paragraph marker([bold]) {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_in_middle_attaches_marker_to_new_paragraph() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("He") [bold] } }
                        list_item { p2: paragraph marker([bold]) { text("llo") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
