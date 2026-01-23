use crate::layout::elements::TableCellElement;
use crate::layout::{
    Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode,
};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tsify::Tsify;

use super::TABLE_CELL_PADDING;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec, Tsify)]
pub struct TableCellNode {
    #[serde(default)]
    pub col_width: Option<f32>,
}

impl Default for TableCellNode {
    fn default() -> Self {
        Self { col_width: None }
    }
}

impl Hash for TableCellNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.col_width.map(|w| w.to_bits()).hash(state);
    }
}

impl NodeHtmlCodec for TableCellNode {
    fn to_dom(&self) -> Option<DomSpec> {
        let mut spec = DomSpec::el("td");
        if let Some(width) = self.col_width {
            spec = spec.attr("data-colwidth", width.to_string());
        }
        Some(spec.hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("td", |elem| {
            let col_width = elem
                .value()
                .attr("data-colwidth")
                .and_then(|s| s.parse().ok());
            Some(Node::TableCell(TableCellNode { col_width }))
        })]
    }
}

impl Layout for TableCellNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;
        let content_width = (max_width - TABLE_CELL_PADDING * 2.0).max(0.0);
        let cell_id = ctx.node.node_id();

        let mut children = Vec::new();
        let mut y = TABLE_CELL_PADDING;

        for child in ctx.node.children() {
            let child_constraints =
                BoxConstraints::new(content_width, content_width, 0.0, f32::MAX);
            let child_layout = ctx.layout(&child, child_constraints);

            children.push(PositionedNode {
                position: Point::new(TABLE_CELL_PADDING, y),
                node: child_layout.clone(),
            });

            y += child_layout.size.height;
        }

        y += TABLE_CELL_PADDING;

        LayoutNode {
            size: Size::new(max_width, y),
            element: Some(Element::TableCell(TableCellElement::new(
                Size::new(max_width, y),
                cell_id,
            ))),
            children: Some(children),
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: Some(cell_id),
        }
    }
}
