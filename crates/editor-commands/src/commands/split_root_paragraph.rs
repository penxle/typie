use editor_crdt::Dot;
use editor_model::NodeType;
use editor_transaction::Transaction;

use crate::helpers::split_paragraph_at_selection;
use crate::{CommandError, CommandResult};

pub fn split_root_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    {
        let view = tr.state().view();
        let paragraph = view
            .node(selection.head.node)
            .ok_or(CommandError::NodeNotFound(selection.head.node))?;
        if paragraph.node_type() != NodeType::Paragraph
            || paragraph
                .parent()
                .is_none_or(|parent| parent.id() != Dot::ROOT)
        {
            return Ok(false);
        }
    }
    split_paragraph_at_selection(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn returns_false_in_list_item() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("item") } }
                }
                paragraph {}
            } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| split_root_paragraph(&mut tr));
    }
}
