use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView};
use editor_state::first_cursor_position;
use editor_state::{Position, Selection};
use editor_transaction::{Transaction, dissolve};

use crate::helpers::{
    child_elem_id, child_node_type, merge_element_cross_parent, next_sibling, prune_empty_real,
    remove_child_at,
};
use crate::{CommandError, CommandResult};

pub fn lift_paragraph_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (
        paragraph_id,
        source_paragraph_id,
        source_parent_id,
        target_trailing_page_break,
        target_children_count,
    ) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }

        let children_len = node.children().count();
        if pos.offset < children_len {
            return Ok(false);
        }
        let paragraph_id = pos.node;

        let next = match next_sibling(&node) {
            Some(ChildView::Block(b)) => b,
            _ => return Ok(false),
        };

        // If next sibling is a paragraph, this is handled by join_paragraph_forward
        if next.node_type() == NodeType::Paragraph {
            return Ok(false);
        }

        let source_paragraph_id = match find_lift_source(&view, &next) {
            Some(id) => id,
            None => return Ok(false),
        };

        let source_parent_id = view
            .node(source_paragraph_id)
            .ok_or(CommandError::NodeNotFound(source_paragraph_id))?
            .parent()
            .ok_or(CommandError::NoParent(source_paragraph_id))?
            .id();

        let last_is_pb = matches!(
            node.last_child(),
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak
        );

        (
            paragraph_id,
            source_paragraph_id,
            source_parent_id,
            last_is_pb,
            children_len,
        )
    };

    let cursor_selection = selection;

    let cursor_on_empty_paragraph = {
        let post_cleanup_count = if target_trailing_page_break {
            target_children_count - 1
        } else {
            target_children_count
        };
        post_cleanup_count == 0
    };

    tr.batch::<_, CommandError>(|tr| {
        // Strip a trailing PageBreak from the target before merging.
        // PageBreak is schema-restricted to the trailing slot; once the
        // source's children are appended the marker would land in the middle.
        let strip_index = {
            let view = tr.state().view();
            view.node(paragraph_id).and_then(|target| {
                let count = target.children().count();
                match target.last_child() {
                    Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak => {
                        Some(count - 1)
                    }
                    _ => None,
                }
            })
        };
        if let Some(idx) = strip_index {
            remove_child_at(tr, paragraph_id, idx)?;
        }

        merge_element_cross_parent(tr, source_paragraph_id, paragraph_id)?;

        let dissolve_steps = {
            let view = tr.state().view();
            match view.node(source_parent_id) {
                Some(source_parent) => {
                    let has_real_child = source_parent
                        .children()
                        .any(|c| child_elem_id(&c).as_op_dot().is_some());
                    if has_real_child {
                        let remaining: Vec<NodeType> = source_parent
                            .children()
                            .map(|c| child_node_type(&c))
                            .collect();
                        if !source_parent.spec().content.matches_sequence(&remaining) {
                            dissolve(&source_parent)
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                }
                None => Vec::new(),
            }
        };
        if dissolve_steps.is_empty() {
            // Empty source container (only a Derived placeholder remains): remove
            // it and cascade to ancestors that empty out as a result.
            prune_empty_real(tr, source_parent_id)?;
        } else {
            tr.apply_steps(dissolve_steps)?;
        }
        Ok(())
    })?;

    if cursor_on_empty_paragraph {
        let restored = {
            let view = tr.state().view();
            view.node(paragraph_id)
                .and_then(|p| first_cursor_position(&p))
        };
        if let Some(pos) = restored {
            tr.set_selection(Some(Selection::collapsed(pos)))?;
            return Ok(true);
        }
    }
    // The strip+merge keeps the target paragraph's content valid, but clamp the
    // captured offsets to the post-edit child count in case the merged content
    // was shorter than the stripped marker.
    let final_selection = {
        let view = tr.state().view();
        let count = view
            .node(paragraph_id)
            .map(|p| p.children().count())
            .unwrap_or(0);
        let clamp = |p: Position| -> Position {
            if p.node == paragraph_id && p.offset > count {
                Position {
                    node: p.node,
                    offset: count,
                    affinity: p.affinity,
                }
            } else {
                p
            }
        };
        Selection::new(clamp(cursor_selection.anchor), clamp(cursor_selection.head))
    };
    tr.set_selection(Some(final_selection))?;

    Ok(true)
}

fn find_lift_source(view: &DocView, container: &NodeView) -> Option<Dot> {
    let mut current_id = container.id();
    loop {
        let current = view.node(current_id)?;
        if current.spec().isolating {
            return None;
        }
        match current.first_child()? {
            ChildView::Block(b) => {
                if b.node_type() == NodeType::Paragraph {
                    return Some(b.id());
                }
                current_id = b.id();
            }
            ChildView::Leaf(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    blockquote { p2: paragraph { text("A") } }
                }
            }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn lift_from_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    blockquote {
                        p2: paragraph { text("A") }
                        p3: paragraph { text("B") }
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloA")
                    }
                    blockquote {
                        p3: paragraph { text("B") }
                    }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_sole_child_removes_container() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    blockquote {
                        p2: paragraph { text("A") }
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloA")
                    }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn next_is_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    blockquote { p2: paragraph { text("A") } }
                }
            }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn empty_paragraph_at_end() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    blockquote {
                        p2: paragraph { text("A") }
                    }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn empty_paragraph_lift_from_callout_selects_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    callout {
                        p2: paragraph { text("Hello, World!") }
                        paragraph { text("안녕하세요!") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello, World!") }
                    callout {
                        paragraph { text("안녕하세요!") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    // GAP F: page_break is an inline atom; the substrate cannot delete it
    // (remove_text is char-count based and no-ops on atoms, remove_subtree
    // rejects non-block children). These two tests exercise trailing-page-break
    // stripping and therefore CANNOT pass until inline-atom deletion lands.
    #[test]
    fn trailing_page_break_dropped_during_lift() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    bullet_list {
                        list_item {
                            paragraph { text("b") }
                        }
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("ab") }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn page_break_only_paragraph_lifts_into_merged_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { page_break }
                    bullet_list {
                        list_item {
                            p2: paragraph { text("b") }
                        }
                    }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
