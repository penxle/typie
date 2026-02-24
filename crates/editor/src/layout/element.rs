use crate::layout::context::LayoutContext;
use crate::layout::cursor::CursorNavigable;
use crate::layout::elements::{Wrapper, *};
use crate::layout::interactive::Interactive;
use crate::model::{NodeId, TABLE_BORDER_WIDTH};
use crate::render::{Outline, Render};
use crate::types::{BoxConstraints, PaintOverflow, Point, PointerStyle, Size};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub use crate::layout::elements::SplitEdges;

pub trait Layout {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageBreakPolicy {
    #[default]
    Auto,
    Avoid,
}

#[derive(Debug, Clone, Default)]
pub struct RenderHints {
    pub default_text_color: Option<String>,
}

impl RenderHints {
    pub fn merge(&self, parent: &RenderHints) -> RenderHints {
        RenderHints {
            default_text_color: self
                .default_text_color
                .clone()
                .or_else(|| parent.default_text_color.clone()),
        }
    }
}

pub struct LayoutNode {
    pub size: Size,
    pub element: Option<Element>,
    pub children: Option<Vec<PositionedNode>>,
    pub page_break_policy: PageBreakPolicy,
    pub render_hints: RenderHints,
    pub scope_id: Option<NodeId>, // TableCell처럼 새 scope를 만드는 경우 그 NodeId
}

#[derive(Clone)]
pub struct PositionedNode {
    pub position: Point,
    pub node: Rc<LayoutNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Line(LineElement),
    External(ExternalElement),
    Blockquote(BlockquoteLineElement),
    BlockquoteQuote(BlockquoteQuoteElement),
    BlockquoteMessage(BlockquoteMessageElement),
    CalloutBackground(CalloutBackgroundElement),
    CalloutIcon(CalloutIconElement),
    HorizontalRule(HorizontalRuleElement),
    ListMarker(ListMarkerElement),
    FoldTitle(FoldTitleElement),
    FoldTitleBackground(FoldTitleBackgroundElement),
    FoldContent(FoldContentElement),
    TableBorder(TableBorderElement),
    TableCell(TableCellElement),
}

impl Element {
    pub fn size(&self) -> Size {
        match self {
            Element::Line(e) => e.size,
            Element::External(e) => e.size,
            Element::Blockquote(e) => e.size,
            Element::BlockquoteQuote(e) => e.size,
            Element::BlockquoteMessage(e) => e.size,
            Element::CalloutBackground(e) => e.size,
            Element::CalloutIcon(e) => e.size,
            Element::HorizontalRule(e) => e.size,
            Element::ListMarker(_) => Size::zero(),
            Element::FoldTitle(e) => e.size,
            Element::FoldTitleBackground(e) => e.size,
            Element::FoldContent(e) => e.size,
            Element::TableBorder(e) => e.size,
            Element::TableCell(e) => e.size,
        }
    }

    pub fn paint_overflow(&self) -> PaintOverflow {
        match self {
            Element::Line(e) => e.paint_overflow(),
            _ => PaintOverflow::default(),
        }
    }

    pub fn as_cursor_navigable(&self) -> Option<&dyn CursorNavigable> {
        match self {
            Element::Line(e) => Some(e),
            Element::External(e) => Some(e),
            Element::Blockquote(_) => None,
            Element::BlockquoteQuote(_) => None,
            Element::BlockquoteMessage(_) => None,
            Element::CalloutBackground(_) => None,
            Element::CalloutIcon(_) => None,
            Element::HorizontalRule(e) => Some(e),
            Element::ListMarker(_) => None,
            Element::FoldTitle(_) => None,
            Element::FoldTitleBackground(_) => None,
            Element::FoldContent(_) => None,
            Element::TableBorder(_) => None,
            Element::TableCell(_) => None,
        }
    }

    pub fn as_render(&self) -> Option<&dyn Render> {
        match self {
            Element::Line(e) => Some(e),
            Element::External(_) => None,
            Element::Blockquote(e) => Some(e),
            Element::BlockquoteQuote(e) => Some(e),
            Element::BlockquoteMessage(e) => Some(e),
            Element::CalloutBackground(e) => Some(e),
            Element::CalloutIcon(e) => Some(e),
            Element::HorizontalRule(e) => Some(e),
            Element::ListMarker(e) => Some(e),
            Element::FoldTitle(e) => Some(e),
            Element::FoldTitleBackground(e) => Some(e),
            Element::FoldContent(e) => Some(e),
            Element::TableBorder(e) => Some(e),
            Element::TableCell(e) => Some(e),
        }
    }

    pub fn as_outline(&self) -> Option<&dyn Outline> {
        match self {
            Element::Line(e) => Some(e),
            Element::External(_) => None,
            Element::Blockquote(e) => Some(e),
            Element::BlockquoteQuote(e) => Some(e),
            Element::BlockquoteMessage(e) => Some(e),
            Element::CalloutBackground(e) => Some(e),
            Element::CalloutIcon(e) => Some(e),
            Element::HorizontalRule(e) => Some(e),
            Element::ListMarker(e) => Some(e),
            Element::FoldTitle(e) => Some(e),
            Element::FoldTitleBackground(e) => Some(e),
            Element::FoldContent(e) => Some(e),
            Element::TableBorder(e) => Some(e),
            Element::TableCell(e) => Some(e),
        }
    }

    pub fn cursor_visual(&self) -> PointerStyle {
        match self {
            Element::Line(_) => PointerStyle::Text,
            Element::External(_) => PointerStyle::Default,
            Element::Blockquote(_) => PointerStyle::Text,
            Element::BlockquoteQuote(_) => PointerStyle::Text,
            Element::BlockquoteMessage(_) => PointerStyle::Text,
            Element::CalloutBackground(_) => PointerStyle::Text,
            Element::CalloutIcon(_) => PointerStyle::Pointer,
            Element::HorizontalRule(_) => PointerStyle::Default,
            Element::ListMarker(_) => PointerStyle::Default,
            Element::FoldTitle(_) => PointerStyle::Pointer,
            Element::FoldTitleBackground(_) => PointerStyle::Pointer,
            Element::FoldContent(_) => PointerStyle::Default,
            Element::TableBorder(_) => PointerStyle::Text,
            Element::TableCell(_) => PointerStyle::Text,
        }
    }

    pub fn block_id(&self) -> Option<NodeId> {
        match self {
            Element::Line(e) => Some(e.block_id),
            Element::External(e) => Some(e.id),
            Element::HorizontalRule(e) => Some(e.node_id),
            Element::Blockquote(e) => Some(e.block_id),
            Element::BlockquoteQuote(e) => Some(e.block_id),
            Element::BlockquoteMessage(e) => Some(e.block_id),
            Element::CalloutBackground(e) => Some(e.node_id),
            Element::CalloutIcon(e) => Some(e.node_id),
            Element::ListMarker(_) => None,
            Element::FoldTitle(e) => Some(e.block_id),
            Element::FoldTitleBackground(e) => Some(e.fold_id),
            Element::FoldContent(e) => Some(e.fold_id),
            Element::TableBorder(e) => Some(e.node_id),
            Element::TableCell(e) => Some(e.node_id),
        }
    }

    pub fn as_interactive(&self) -> Option<&dyn Interactive> {
        match self {
            Element::FoldTitle(e) => Some(e),
            Element::FoldTitleBackground(e) => Some(e),
            Element::CalloutIcon(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_wrapper(&self) -> Option<&dyn Wrapper> {
        match self {
            Element::CalloutBackground(e) => Some(e),
            Element::BlockquoteMessage(e) => Some(e),
            Element::FoldContent(e) => Some(e),
            Element::TableBorder(e) => Some(e),
            _ => None,
        }
    }

    pub fn hash_render_cache_signature<H: Hasher>(&self, state: &mut H) -> bool {
        match self {
            Element::CalloutBackground(e) => {
                e.hash(state);
                true
            }
            Element::BlockquoteMessage(e) => {
                e.hash(state);
                true
            }
            Element::FoldContent(e) => {
                e.hash(state);
                true
            }
            Element::TableBorder(e) => {
                e.hash(state);
                true
            }
            Element::Line(_)
            | Element::External(_)
            | Element::Blockquote(_)
            | Element::BlockquoteQuote(_)
            | Element::CalloutIcon(_)
            | Element::HorizontalRule(_)
            | Element::ListMarker(_)
            | Element::FoldTitle(_)
            | Element::FoldTitleBackground(_)
            | Element::TableCell(_) => false,
        }
    }

    pub fn with_adjusted_bounds(
        &self,
        new_height: f32,
        split_edges: SplitEdges,
    ) -> Option<Element> {
        match self {
            Element::CalloutBackground(e) => {
                Some(Element::CalloutBackground(CalloutBackgroundElement::new(
                    Size::new(e.size.width, new_height),
                    e.variant,
                    e.node_id,
                    split_edges,
                )))
            }
            Element::BlockquoteMessage(e) => {
                Some(Element::BlockquoteMessage(BlockquoteMessageElement::new(
                    Size::new(e.size.width, new_height),
                    e.block_id,
                    e.variant,
                    split_edges,
                )))
            }
            Element::FoldContent(e) => Some(Element::FoldContent(FoldContentElement::new(
                Size::new(e.size.width, new_height),
                split_edges,
                e.fold_id,
            ))),
            Element::TableBorder(e) => {
                let offset = if split_edges.top {
                    e.size.height - new_height
                } else {
                    e.offset
                };

                let mut accumulated_height = if split_edges.top {
                    0.0
                } else {
                    TABLE_BORDER_WIDTH
                };
                let mut start_idx = 0;
                let mut _end_idx = 0;
                let mut current_offset = 0.0;

                for (i, &h) in e.row_heights.iter().enumerate() {
                    if current_offset + h > offset {
                        start_idx = i;
                        break;
                    }
                    current_offset += h;
                }

                let mut sliced_heights = Vec::new();
                for &h in e.row_heights.iter().skip(start_idx) {
                    if accumulated_height + h <= new_height + 0.1 {
                        sliced_heights.push(h);
                        accumulated_height += h;
                    } else {
                        break;
                    }
                }

                let new_start_row_index = e.start_row_index + start_idx;

                if !split_edges.bottom {
                    accumulated_height += TABLE_BORDER_WIDTH;
                }

                if sliced_heights.is_empty() && !e.row_heights.is_empty() {
                    return None;
                }

                Some(Element::TableBorder(TableBorderElement::new(
                    Size::new(e.size.width, accumulated_height),
                    e.node_id,
                    e.border_style,
                    e.align,
                    sliced_heights.len(),
                    e.cols,
                    sliced_heights,
                    e.col_widths.clone(),
                    split_edges,
                    offset,
                    e.x_offset,
                    new_start_row_index,
                    e.total_rows,
                )))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::elements::{CalloutBackgroundElement, FoldContentElement};
    use crate::model::{CalloutVariant, NodeId};

    fn all_wrapper_elements() -> Vec<Element> {
        vec![
            Element::CalloutBackground(CalloutBackgroundElement::new(
                Size::new(100.0, 100.0),
                CalloutVariant::Info,
                NodeId::new(),
                SplitEdges::default(),
            )),
            Element::FoldContent(FoldContentElement::new(
                Size::new(100.0, 100.0),
                SplitEdges::default(),
                NodeId::new(),
            )),
        ]
    }

    #[test]
    fn all_wrappers_support_height_adjustment() {
        for element in all_wrapper_elements() {
            assert!(
                element.as_wrapper().is_some(),
                "{:?} should be a wrapper",
                element
            );
            assert!(
                element
                    .with_adjusted_bounds(200.0, SplitEdges::default())
                    .is_some(),
                "{:?} should support with_adjusted_bounds",
                element
            );
        }
    }
}
