use super::*;
use crate::diagnostics::LayoutPassRecorder;
use crate::layout::elements::{
    BackgroundSegment, BlockquoteLineElement, BlockquoteMessageElement, BlockquoteQuoteElement,
    CalloutBackgroundElement, CalloutIconElement, FoldContentElement, LineElement, LineMetric,
    ListMarkerElement, ListMarkerType, RubySegment, SplitEdges, TableBorderElement,
    TableCellElement,
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
    marker_node_for(NodeId::new(), size)
}

fn marker_node_for(selection_node_id: NodeId, size: Size) -> Rc<LayoutNode> {
    Rc::new(LayoutNode {
        size,
        element: Some(Element::ListMarker(ListMarkerElement::new(
            ListMarkerType::Bullet,
            8.0,
            6.0,
            size.width.min(crate::model::LIST_ITEM_MARKER_WIDTH),
            selection_node_id,
            size.width,
            size.height,
        ))),
        children: None,
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    })
}

fn line_node(block_id: NodeId, text: &str, size: Size) -> Rc<LayoutNode> {
    let text_len = text.chars().count();
    let grapheme_offsets = if text_len == 0 {
        vec![0]
    } else {
        vec![0, text_len]
    };

    Rc::new(LayoutNode {
        size,
        element: Some(Element::Line(LineElement::build(
            block_id,
            size,
            0,
            Rc::new(parley::Layout::default()),
            LineMetric {
                top: 0.0,
                left: 0.0,
                height: size.height,
                leading: 0.0,
                baseline: (size.height * 0.75).round(),
                ascent: (size.height * 0.75).round(),
                content_width: size.width,
                start_offset: 0,
                end_offset: text_len,
                clusters: vec![],
                break_reason: parley::layout::BreakReason::None,
                grapheme_offsets,
                ascent_overflow: 0.0,
                descent_overflow: 0.0,
            },
            None,
            text_len == 0,
            Rc::from(text),
            vec![],
            vec![],
            false,
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
