use editor_model::Node;
use editor_transaction::Transaction;

use crate::helpers::{LiftDirection, lift};
use crate::{CommandError, CommandResult};

pub fn lift_last_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let paragraph_id = match node.node() {
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

    if paragraph.next_sibling().is_some() || paragraph.first_child().is_some() {
        return Ok(false);
    }

    lift(tr, paragraph_id, LiftDirection::End)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_empty_returns_false() {
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
        transact_fail!(initial, |tr| lift_last_paragraph(&mut tr));
    }

    #[test]
    fn not_last_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p1: paragraph {}
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_last_paragraph(&mut tr));
    }

    #[test]
    fn isolating_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("Title") }
                        fold_content {
                            paragraph { text("A") }
                            p1: paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_last_paragraph(&mut tr));
    }

    #[test]
    fn content_spec_mismatch_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            p1: paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_last_paragraph(&mut tr));
    }

    #[test]
    fn empty_last_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_last_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                    }
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn from_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { text("A") }
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_last_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { text("A") }
                    }
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
