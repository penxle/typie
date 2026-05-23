use editor_model::{Node, NodeId, PlainNode, PlainTableNode, TableBorderStyle};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_table_border_style(
    tr: &mut Transaction,
    table_id: NodeId,
    border_style: TableBorderStyle,
) -> CommandResult {
    let proportion = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        match table.node() {
            Node::Table(n) => *n.proportion.get(),
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

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_border_style() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_border_style(
            &mut tr,
            tbl,
            TableBorderStyle::Dashed
        ));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        if let Node::Table(n) = table.node() {
            assert_eq!(*n.border_style.get(), TableBorderStyle::Dashed);
        } else {
            panic!("expected Table node");
        }
    }

    #[test]
    fn preserves_proportion_when_changing_border() {
        use crate::commands::set_table_proportion::set_table_proportion;

        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (after_proportion, ..) =
            transact!(initial.clone(), |tr| set_table_proportion(&mut tr, tbl, 75));
        let (actual, ..) = transact!(after_proportion, |tr| set_table_border_style(
            &mut tr,
            tbl,
            TableBorderStyle::Dotted
        ));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        if let Node::Table(n) = table.node() {
            assert_eq!(*n.proportion.get(), 75);
            assert_eq!(*n.border_style.get(), TableBorderStyle::Dotted);
        } else {
            panic!("expected Table node");
        }
    }
}
