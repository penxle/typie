use super::*;
use crate::diagnostics::LayoutPassRecorder;
use crate::layout::elements::{
    BackgroundSegment, BlockquoteLineElement, BlockquoteMessageElement, CalloutBackgroundElement,
    CalloutIconElement, FoldContentElement, LineElement, LineMetric, ListMarkerElement,
    ListMarkerType, RubySegment, SplitEdges, TableBorderElement, TableCellElement,
};
use crate::layout::{LayoutNode, PageBreakPolicy};
use crate::model::{BlockquoteVariant, CalloutVariant, NodeId, TableAlign, TableBorderStyle};
use crate::types::Size;
use rustc_hash::{FxHashMap, FxHashSet};
use std::rc::Rc;

fn root_with_children(children: Option<Vec<PositionedNode>>, size: Size) -> Page {
    Page::from_root(PositionedNode {
        position: Point::zero(),
        node: Rc::new(LayoutNode {
            size,
            element: None,
            children,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        }),
    })
}

fn marker_node(size: Size) -> Rc<LayoutNode> {
    Rc::new(LayoutNode {
        size,
        element: Some(Element::ListMarker(ListMarkerElement::new(
            ListMarkerType::Bullet,
            8.0,
            6.0,
            size.width,
        ))),
        children: None,
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    })
}

fn callout_page_with_icon(callout_id: NodeId) -> Page {
    let icon_node = Rc::new(LayoutNode {
        size: Size::new(20.0, 20.0),
        element: Some(Element::CalloutIcon(CalloutIconElement::new(
            Size::new(20.0, 20.0),
            CalloutVariant::Info,
            callout_id,
        ))),
        children: None,
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    });

    let callout_node = Rc::new(LayoutNode {
        size: Size::new(140.0, 80.0),
        element: Some(Element::CalloutBackground(CalloutBackgroundElement::new(
            Size::new(140.0, 80.0),
            CalloutVariant::Info,
            callout_id,
            SplitEdges::default(),
        ))),
        children: Some(vec![PositionedNode {
            position: Point::new(12.0, 12.0),
            node: icon_node,
        }]),
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    });

    root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: callout_node,
        }]),
        Size::new(220.0, 160.0),
    )
}

fn rgba_at(buf: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let idx = (y * width + x) * 4;
    [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
}

mod cache;
mod horizontal_rule;
mod layout_debug;
mod overflow;
mod overlay;
mod snapshot;
