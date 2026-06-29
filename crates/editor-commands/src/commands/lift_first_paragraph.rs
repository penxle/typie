use editor_model::NodeType;
use editor_transaction::Transaction;

use crate::helpers::{LiftDirection, lift};
use crate::{CommandError, CommandResult};

pub fn lift_first_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset > 0 {
        return Ok(false);
    }

    let paragraph_id = pos.node;

    {
        let view = tr.state().view();
        let paragraph = view
            .node(paragraph_id)
            .ok_or(CommandError::NodeNotFound(paragraph_id))?;

        if paragraph.node_type() != NodeType::Paragraph {
            return Ok(false);
        }

        let parent = paragraph
            .parent()
            .ok_or(CommandError::NoParent(paragraph_id))?;

        if parent.child_blocks().position(|b| b.id() == paragraph_id) != Some(0) {
            return Ok(false);
        }
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
                    blockquote { p1: paragraph { text("A") } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
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
                        p1: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn parent_is_root_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn list_item_filtered_by_content_spec() {
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
        transact_fail!(initial, |tr| lift_first_paragraph(&mut tr));
    }

    #[test]
    fn lift_from_blockquote_multiple_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        paragraph { text("B") }
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
                    p1: paragraph { text("A") }
                    blockquote {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        p1: paragraph { text("A") }
                        paragraph { text("B") }
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
                    p1: paragraph { text("A") }
                    callout {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sole_child_prunes_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
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
                    p1: paragraph { text("A") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
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
                        p1: paragraph { text("Hello") }
                        paragraph { text("World") }
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
                    p1: paragraph { text("Hello") }
                    blockquote {
                        paragraph { text("World") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
