use editor_model::{ChildView, NodeType};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn join_paragraph_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let paragraph_id = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let child_count = node.children().count();
        if pos.offset < child_count {
            return Ok(false);
        }
        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        // Index by the full child list: a block-level atom (image/HR) next to the
        // paragraph projects as a leaf and would be skipped by child_blocks(),
        // and an invalid trailing container is patched with a Derived paragraph —
        // both would mislead a block-only lookup into a bogus merge.
        let index = node
            .index()
            .ok_or_else(|| CommandError::orphan_child(pos.node, parent.id()))?;
        match parent.child_at(index + 1) {
            Some(ChildView::Block(b))
                if b.node_type() == NodeType::Paragraph && b.dot().is_some() => {}
            _ => return Ok(false),
        }
        pos.node
    };

    tr.merge_node(paragraph_id)?;

    tr.set_selection(Some(selection))?;

    Ok(true)
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
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn join_two_text_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloWorld")
                    }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_empty_next() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_empty_current() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    p2: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_both_empty() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
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
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn next_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    horizontal_rule
                }
            }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn join_preserves_all_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("A")
                        text("B") [bold]
                    }
                    paragraph {
                        text("CD")
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("A")
                        text("B") [bold]
                        text("CD")
                    }
                }
            }
            selection: (m, 2)
        };
        assert_state_eq!(&actual, &expected);
    }
}
