use editor_common::{Alignment, EdgeInsets};
use editor_model::{Doc, Node, NodeRef};

use crate::measure::container::layout_padded;
use crate::measure::*;
use crate::view_state::ViewState;

use crate::measure::Measurer;

use super::atom::measure_atom;
use super::blockquote::measure_blockquote;
use super::callout::measure_callout;
use super::fold::{measure_fold, measure_fold_content, measure_fold_title};
use super::list_item::measure_list_item;
use super::paragraph::measure::measure_paragraph;
use super::table::{measure_table, measure_table_cell};

pub(crate) fn measure_node(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    match node.node() {
        Node::Paragraph(_) => measure_paragraph(measurer, doc, node, width),
        Node::ListItem(_) => measure_list_item(measurer, doc, node, width, view_state),
        Node::Blockquote(_) => measure_blockquote(measurer, doc, node, width, view_state),
        Node::Callout(_) => measure_callout(measurer, doc, node, width, view_state),
        Node::Fold(_) => measure_fold(measurer, doc, node, width, view_state),
        Node::FoldTitle(_) => measure_fold_title(measurer, doc, node, width, view_state),
        Node::FoldContent(_) => measure_fold_content(measurer, doc, node, width, view_state),
        Node::Table(_) => measure_table(measurer, doc, node, width, view_state),
        Node::TableCell(_) => measure_table_cell(measurer, doc, node, width, view_state),
        Node::Image(_)
        | Node::File(_)
        | Node::Embed(_)
        | Node::Archived(_)
        | Node::HorizontalRule(_) => measure_atom(node, width, view_state),
        Node::PageBreak(_) => MeasuredNode {
            width,
            height: 0.0,
            content: MeasuredContent::PageBreak,
        },
        _ => layout_padded(
            measurer,
            doc,
            node,
            width,
            view_state,
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            false,
            Alignment::Start,
        ),
    }
}

#[cfg(test)]
mod integration_tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use crate::measure::Measurer;
    use crate::measure::*;
    use crate::view_state::ViewState;

    #[test]
    fn compute_full_document() {
        let (doc,) = doc! {
            root {
                paragraph { text("Hello") }
                paragraph { text("World") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);
        let MeasuredContent::Box(b) = &root.content else {
            panic!()
        };
        assert!(b.children.len() >= 2);
        assert!(root.height > 0.0);
    }

    #[test]
    fn measure_with_blockquote_and_paragraph() {
        let (doc,) = doc! {
            root {
                paragraph { text("Before") }
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { text("Quoted") }
                }
                paragraph { text("After") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);
        let MeasuredContent::Box(b) = &root.content else {
            panic!()
        };
        let box_count = b
            .children
            .iter()
            .filter(|c| matches!(c.content, MeasuredContent::Box(_)))
            .count();
        assert_eq!(box_count, 3);
    }
}
