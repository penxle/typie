use editor_model::{Node, NodeId, PlainNode, PlainTableNode};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_table_proportion(
    tr: &mut Transaction,
    table_id: NodeId,
    proportion: u32,
) -> CommandResult {
    let border_style = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        match table.node() {
            Node::Table(n) => *n.border_style.get(),
            _ => return Err(CommandError::NodeNotFound(table_id)),
        }
    };
    tr.set_node(
        table_id,
        PlainNode::Table(PlainTableNode {
            border_style,
            proportion,
        }),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::TableBorderStyle;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_proportion() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_proportion(&mut tr, tbl, 75));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        if let Node::Table(n) = table.node() {
            assert_eq!(*n.proportion.get(), 75);
        } else {
            panic!("expected Table node");
        }
    }

    #[test]
    fn preserves_border_style_when_changing_proportion() {
        use crate::commands::set_table_border_style::set_table_border_style;

        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (after_border, ..) = transact!(initial.clone(), |tr| set_table_border_style(
            &mut tr,
            tbl,
            TableBorderStyle::Dashed
        ));
        let (actual, ..) = transact!(after_border, |tr| set_table_proportion(&mut tr, tbl, 50));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        if let Node::Table(n) = table.node() {
            assert_eq!(*n.proportion.get(), 50);
            assert_eq!(*n.border_style.get(), TableBorderStyle::Dashed);
        } else {
            panic!("expected Table node");
        }
    }
}
