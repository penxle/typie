use std::sync::Arc;

use editor_common::EdgeInsets;

use crate::style::Alignment;
use editor_model::{Doc, Modifier, ModifierType, NodeRef};

use crate::measure::Measurer;
use crate::measure::resolve::resolve_inherited;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Direction};
use crate::view_state::ViewState;

const BLOCK_GAP_BASE_PX: f32 = 16.0;

pub fn resolve_gap_after(node: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node, ModifierType::BlockGap) {
        Some(Modifier::BlockGap { value }) => *value as f32 / 100.0 * BLOCK_GAP_BASE_PX,
        _ => 0.0,
    }
}

pub fn layout_vertical(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> (Vec<Arc<MeasuredNode>>, f32) {
    let children_refs: Vec<_> = node.children().collect();
    let mut result = Vec::new();
    let mut total_height = 0.0;

    for (i, child) in children_refs.iter().enumerate() {
        let m = measurer.measure(doc, child.id(), width, view_state);
        total_height += m.height;
        result.push(m);

        if i < children_refs.len() - 1 {
            let child_node = doc.node(child.id()).unwrap();
            let gap = resolve_gap_after(&child_node);
            if gap > 0.0 {
                result.push(Arc::new(MeasuredNode {
                    width,
                    height: gap,
                    content: MeasuredContent::Spacing(gap),
                }));
                total_height += gap;
            }
        }
    }

    (result, total_height)
}

pub struct PaddedLayoutConfig {
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub scope: bool,
    pub alignment: Alignment,
}

pub fn layout_padded(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
    config: PaddedLayoutConfig,
) -> MeasuredNode {
    let PaddedLayoutConfig {
        padding,
        border,
        scope,
        alignment,
    } = config;
    let inner_width = width - padding.left - padding.right - border.left - border.right;
    let (children, children_height) = layout_vertical(measurer, doc, node, inner_width, view_state);
    let total_height = children_height + padding.top + padding.bottom + border.top + border.bottom;

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border,
                border_mode: BorderMode::Separate,
                alignment,
                scope,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn sums_children() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph { text("hello") }
            }
        };

        let node = doc.node(p1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = layout_padded(
            &mut measurer,
            &doc,
            &node,
            300.0,
            &ViewState::new(),
            PaddedLayoutConfig {
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                scope: false,
                alignment: Alignment::Start,
            },
        );

        assert!(matches!(result.content, MeasuredContent::Box(_)));
        assert_eq!(result.width, 300.0);
    }

    #[test]
    fn inserts_gap_as_spacing() {
        let (doc,) = doc! {
            root [block_gap(200)] {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };

        let node = doc.node(NodeId::ROOT).unwrap();
        let mut measurer = Measurer::new_test();
        let (children, _) = layout_vertical(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        assert_eq!(children.len(), 3);
        assert!(matches!(children[1].content, MeasuredContent::Spacing(_)));
    }

    #[test]
    fn resolve_gap_after_converts_block_gap() {
        let (doc, p1) = doc! { root [block_gap(100)] { p1: paragraph } };
        let node = doc.node(p1).unwrap();
        assert_eq!(resolve_gap_after(&node), 16.0);
    }

    #[test]
    fn resolve_gap_after_returns_zero_when_no_block_gap() {
        let (doc, p1) = doc! { root [] { p1: paragraph } };
        let node = doc.node(p1).unwrap();
        assert_eq!(resolve_gap_after(&node), 0.0);
    }
}
