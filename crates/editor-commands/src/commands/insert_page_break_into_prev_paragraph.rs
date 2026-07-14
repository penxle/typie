use editor_model::{ChildView, Node};
use editor_transaction::Transaction;

use crate::helpers::{
    find_ancestor_textblock, insert_terminal_page_break_into_root_paragraph, prev_sibling,
};
use crate::{CommandError, CommandResult};

pub fn insert_page_break_into_prev_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;

    let prev_id = {
        let view = tr.state().view();
        let Some(paragraph_id) = find_ancestor_textblock(&view, pos.node) else {
            return Ok(false);
        };
        let paragraph = view
            .node(paragraph_id)
            .ok_or(CommandError::NodeNotFound(paragraph_id))?;

        if !matches!(paragraph.node(), Node::Paragraph(_)) {
            return Ok(false);
        }
        if paragraph
            .parent()
            .is_none_or(|parent| parent.id() != view.root().unwrap().id())
        {
            return Ok(false);
        }

        paragraph
            .parent()
            .ok_or(CommandError::NoParent(paragraph_id))?;
        let prev = match prev_sibling(&paragraph) {
            Some(ChildView::Block(prev)) => prev,
            _ => return Ok(false),
        };
        if !matches!(prev.node(), Node::Paragraph(_)) {
            return Ok(false);
        }
        prev.id()
    };

    insert_terminal_page_break_into_root_paragraph(tr, prev_id)
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
                    paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
            }
            selection: (p2, 0) -> (p2, 3)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn prev_sibling_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { text("a") } }
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn image_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    image
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn current_paragraph_not_root_child_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("a") }
                        p1: paragraph { text("b") }
                    }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn prev_already_has_page_break_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn current_textblock_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    fold {
                        ft1: fold_title { text("title") }
                        fold_content { paragraph {} }
                    }
                }
            }
            selection: (ft1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn inserts_into_prev_paragraph_with_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_into_empty_prev_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph {} p2: paragraph {} } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { page_break {} } p2: paragraph {} } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_when_cursor_in_middle_of_current_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    p1: paragraph { text("world") }
                }
            }
            selection: (p1, 0)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }
}
