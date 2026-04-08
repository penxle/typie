use editor_model::Node;
use editor_transaction::Transaction;

use crate::helpers::{LiftDirection, lift};
use crate::{CommandError, CommandResult};

pub fn lift_first_paragraph(tr: &mut Transaction) -> CommandResult {
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
        Node::Text(_) => {
            if pos.offset > 0 || node.prev_sibling().is_some() {
                return Ok(false);
            }
            node.parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id()
        }
        Node::Paragraph(_) => {
            if pos.offset > 0 {
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

    if paragraph.prev_sibling().is_some() {
        return Ok(false);
    }

    lift(tr, paragraph_id, LiftDirection::Front)
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
                    blockquote { paragraph { t1: text("A") } }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn not_first_child_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        paragraph { t1: text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn parent_is_root_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn list_item_filtered_by_content_spec() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn lift_from_blockquote_multiple_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    blockquote {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { t1: text("A") }
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    callout {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sole_child_prunes_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_empty_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cursor_on_text_node_at_start() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("Hello") }
                        paragraph { text("World") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    blockquote {
                        paragraph { text("World") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
