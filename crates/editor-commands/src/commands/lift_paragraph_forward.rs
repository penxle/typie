use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId, NodeRef, NodeType};
use editor_state::Selection;
use editor_transaction::{Transaction, compact, dissolve, prune};

use crate::helpers::find_first_cursor_position;
use crate::{CommandError, CommandResult};

pub fn lift_paragraph_forward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let paragraph_id = match node.node() {
        Node::Text(text_node) => {
            let text_len = text_node.text.char_count();
            if pos.offset < text_len || node.next_sibling().is_some() {
                return Ok(false);
            }
            node.parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id()
        }
        Node::Paragraph(_) => {
            let children_len = node.entry().children.len();
            if pos.offset < children_len {
                return Ok(false);
            }
            pos.node_id
        }
        _ => return Ok(false),
    };

    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let next = match paragraph.next_sibling() {
        Some(next) => next,
        None => return Ok(false),
    };

    // If next sibling is a paragraph, this is handled by join_paragraph_forward
    if matches!(next.node(), Node::Paragraph(_)) {
        return Ok(false);
    }

    let source_paragraph_id = match find_lift_source(&doc, &next) {
        Some(id) => id,
        None => return Ok(false),
    };

    // Record the parent of the source paragraph (for post-merge cleanup)
    let source_parent_id = doc
        .node(source_paragraph_id)
        .ok_or(CommandError::NodeNotFound(source_paragraph_id))?
        .parent()
        .ok_or(CommandError::NoParent(source_paragraph_id))?
        .id();

    let cursor_selection = tr.selection();
    let cursor_on_empty_paragraph =
        matches!(node.node(), Node::Paragraph(_)) && node.entry().children.is_empty();

    tr.batch::<_, CommandError>(|tr| {
        tr.merge_node(source_paragraph_id, paragraph_id)?;

        let doc = tr.doc();
        if let Some(p) = doc.node(paragraph_id) {
            tr.apply_steps(compact(&p))?;
        }

        let doc = tr.doc();
        if let Some(source_parent) = doc.node(source_parent_id) {
            let remaining: Vec<NodeType> = source_parent.children().map(|c| c.as_type()).collect();

            if source_parent.entry().children.is_empty() {
                tr.apply_steps(prune(&source_parent))?;
            } else if !source_parent
                .node()
                .spec()
                .content
                .matches_sequence(&remaining)
            {
                tr.apply_steps(dissolve(&source_parent))?;
            }
        }
        Ok(())
    })?;

    if cursor_on_empty_paragraph {
        let doc = tr.doc();
        if let Some(p) = doc.node(paragraph_id) {
            if let Some(pos) = find_first_cursor_position(&p) {
                tr.set_selection(Selection::collapsed(pos))?;
                return Ok(true);
            }
        }
    }
    tr.set_selection(cursor_selection)?;

    Ok(true)
}

fn find_lift_source(doc: &Doc, container: &NodeRef) -> Option<NodeId> {
    let mut current_id = container.id();
    loop {
        let current = doc.node(current_id)?;
        let spec = current.spec();
        if spec.isolating {
            return None;
        }
        let first_child = current.first_child()?;
        if matches!(first_child.node(), Node::Paragraph(_)) {
            return Some(first_child.id());
        }
        current_id = first_child.id();
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
                    paragraph { t1: text("Hello") }
                    blockquote { paragraph { t2: text("A") } }
                }
            }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn lift_from_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    blockquote {
                        paragraph { t2: text("A") }
                        paragraph { t3: text("B") }
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("HelloA")
                    }
                    blockquote {
                        paragraph { t3: text("B") }
                    }
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_sole_child_removes_container() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    blockquote {
                        paragraph { t2: text("A") }
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("HelloA")
                    }
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn next_is_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| lift_paragraph_forward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    blockquote { paragraph { t2: text("A") } }
                }
            }
            selection: (t1, 3)
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
                        paragraph { t1: text("A") }
                    }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
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
                        paragraph { t1: text("Hello, World!") }
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
                    paragraph { t1: text("Hello, World!") }
                    callout {
                        paragraph { text("안녕하세요!") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
