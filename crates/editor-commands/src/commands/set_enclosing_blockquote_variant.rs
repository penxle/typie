use editor_model::{BlockquoteVariant, Node, NodeType, PlainBlockquoteNode, PlainNode};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{promote_list_run, resolve_selected_block_run_for_wrapper};

pub fn set_enclosing_blockquote_variant(
    tr: &mut Transaction,
    variant: BlockquoteVariant,
) -> CommandResult {
    let Some(run) = resolve_selected_block_run_for_wrapper(tr, NodeType::Blockquote)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    let wrapper_id = run.parent_id;
    {
        let view = tr.view();
        let Some(wrapper) = view.node(wrapper_id) else {
            return Ok(false);
        };
        let Node::Blockquote(blockquote) = wrapper.node() else {
            return Ok(false);
        };
        if *blockquote.variant.get() == variant {
            return Ok(false);
        }
    }
    tr.set_node(
        wrapper_id,
        PlainNode::Blockquote(PlainBlockquoteNode { variant }),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::BlockquoteVariant;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn changes_enclosing_variant_and_preserves_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("Hello") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1) -> (p1, 4)
        };

        let (actual, ..) = transact!(initial, |tr| set_enclosing_blockquote_variant(
            &mut tr,
            BlockquoteVariant::LeftQuote,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::LeftQuote) {
                        p1: paragraph { text("Hello") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }
}
