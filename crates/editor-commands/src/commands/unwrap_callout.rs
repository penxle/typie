use editor_model::{Node, NodeId};
use editor_transaction::Transaction;

use crate::helpers::unwrap_block_wrapper;
use crate::{CommandError, CommandResult};

pub fn unwrap_callout(tr: &mut Transaction, node_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let node = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;
    if !matches!(node.node(), Node::Callout(_)) {
        return Ok(false);
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
                        paragraph { t: text("hello") }
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
                    paragraph { t: text("hello") }
                    paragraph {}
                }
            }
            selection: (t, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_callout_with_multiple_children() {
        let (initial, co, ..) = state! {
            doc {
                root {
                    co: callout {
                        paragraph { t1: text("a") }
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
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                    ordered_list {
                        list_item { paragraph { text("c") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
