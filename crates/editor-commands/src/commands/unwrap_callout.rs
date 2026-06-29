use editor_crdt::Dot;
use editor_model::Node;
use editor_transaction::Transaction;

use crate::helpers::unwrap_block_wrapper;
use crate::{CommandError, CommandResult};

pub fn unwrap_callout(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    {
        let view = tr.view();
        let node = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        if !matches!(node.node(), Node::Callout(_)) {
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
    fn unwrap_callout_with_single_paragraph() {
        let (initial, co, ..) = state! {
            doc {
                root {
                    co: callout {
                        p1: paragraph { text("hello") }
                    }
                    paragraph {}
                }
            }
            selection: (co, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_callout(&mut tr, co));
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
    fn unwrap_callout_with_multiple_children() {
        let (initial, co, ..) = state! {
            doc {
                root {
                    co: callout {
                        p1: paragraph { text("a") }
                        paragraph { text("b") }
                        ordered_list {
                            list_item { paragraph { text("c") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (co, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_callout(&mut tr, co));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    paragraph { text("b") }
                    ordered_list {
                        list_item { paragraph { text("c") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_non_callout_returns_false() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    bq: blockquote { paragraph { text("hi") } }
                    paragraph {}
                }
            }
            selection: (bq, 0)
        };
        transact_fail!(initial, |tr| unwrap_callout(&mut tr, bq));
    }
}
