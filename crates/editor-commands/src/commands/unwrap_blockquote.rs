use editor_crdt::Dot;
use editor_model::Node;
use editor_transaction::Transaction;

use crate::helpers::unwrap_block_wrapper;
use crate::{CommandError, CommandResult};

pub fn unwrap_blockquote(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    {
        let view = tr.view();
        let node = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        if !matches!(node.node(), Node::Blockquote(_)) {
            return Ok(false);
        }
    }
    unwrap_block_wrapper(tr, node_id)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn unwrap_blockquote_with_single_paragraph() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    bq: blockquote {
                        p1: paragraph { text("hello") }
                    }
                    paragraph {}
                }
            }
            selection: (bq, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_blockquote(&mut tr, bq));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_blockquote_with_multiple_children() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    bq: blockquote {
                        p1: paragraph { text("a") }
                        paragraph { text("b") }
                    }
                    paragraph {}
                }
            }
            selection: (bq, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_blockquote(&mut tr, bq));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    paragraph { text("b") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_blockquote_preserves_nested_list() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    bq: blockquote {
                        p1: paragraph { text("a") }
                        bullet_list {
                            list_item { paragraph { text("b") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (bq, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_blockquote(&mut tr, bq));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    bullet_list {
                        list_item { paragraph { text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_non_blockquote_returns_false() {
        let (initial, p, ..) = state! {
            doc {
                root {
                    p: paragraph { text("hi") }
                }
            }
            selection: (p, 0)
        };
        transact_fail!(initial, |tr| unwrap_blockquote(&mut tr, p));
    }

    #[test]
    fn unwrap_blockquote_inside_fold_content() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content {
                            bq: blockquote {
                                p1: paragraph { text("body") }
                            }
                        }
                    }
                }
            }
            selection: (bq, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_blockquote(&mut tr, bq));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content {
                            p1: paragraph { text("body") }
                        }
                    }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
