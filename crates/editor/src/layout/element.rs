use crate::layout::context::LayoutContext;
use crate::layout::cursor::CursorNavigable;
use crate::layout::elements::*;
use crate::layout::interactive::Interactive;
use crate::render::Render;
use crate::types::{BoxConstraints, Point, PointerStyle, Size};
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

pub struct LayoutNode {
    pub size: Size,
    pub element: Option<Element>,
    pub children: Option<Vec<PositionedNode>>,
    pub page_break_policy: PageBreakPolicy,
}

pub struct PositionedNode {
    pub position: Point,
    pub node: Rc<LayoutNode>,
}

#[derive(Debug, Clone)]
pub enum Element {
    Line(LineElement),
    External(ExternalElement),
    Blockquote(BlockquoteLineElement),
    CalloutBackground(CalloutBackgroundElement),
    CalloutIcon(CalloutIconElement),
    HorizontalRule(HorizontalRuleElement),
    ListMarker(ListMarkerElement),
    FoldTitle(FoldTitleElement),
    FoldTitleBackground(FoldTitleBackgroundElement),
    FoldContent(FoldContentElement),
}

impl Element {
    pub fn size(&self) -> Size {
        match self {
            Element::Line(e) => e.size,
            Element::External(e) => e.size,
            Element::Blockquote(e) => e.size,
            Element::CalloutBackground(e) => e.size,
            Element::CalloutIcon(e) => e.size,
            Element::HorizontalRule(e) => e.size,
            Element::ListMarker(_) => Size::zero(),
            Element::FoldTitle(e) => e.size,
            Element::FoldTitleBackground(e) => e.size,
            Element::FoldContent(e) => e.size,
        }
    }

    pub fn as_cursor_navigable(&self) -> Option<&dyn CursorNavigable> {
        match self {
            Element::Line(e) => Some(e),
            Element::External(e) => Some(e),
            Element::Blockquote(_) => None,
            Element::CalloutBackground(_) => None,
            Element::CalloutIcon(_) => None,
            Element::HorizontalRule(e) => Some(e),
            Element::ListMarker(_) => None,
            Element::FoldTitle(_) => None,
            Element::FoldTitleBackground(_) => None,
            Element::FoldContent(_) => None,
        }
    }

    pub fn as_render(&self) -> Option<&dyn Render> {
        match self {
            Element::Line(e) => Some(e),
            Element::External(_) => None,
            Element::Blockquote(e) => Some(e),
            Element::CalloutBackground(e) => Some(e),
            Element::CalloutIcon(e) => Some(e),
            Element::HorizontalRule(e) => Some(e),
            Element::ListMarker(e) => Some(e),
            Element::FoldTitle(e) => Some(e),
            Element::FoldTitleBackground(e) => Some(e),
            Element::FoldContent(e) => Some(e),
        }
    }

    pub fn cursor_visual(&self) -> PointerStyle {
        match self {
            Element::Line(_) => PointerStyle::Text,
            Element::External(_) => PointerStyle::Default,
            Element::Blockquote(_) => PointerStyle::Text,
            Element::CalloutBackground(_) => PointerStyle::Text,
            Element::CalloutIcon(_) => PointerStyle::Pointer,
            Element::HorizontalRule(_) => PointerStyle::Default,
            Element::ListMarker(_) => PointerStyle::Default,
            Element::FoldTitle(_) => PointerStyle::Pointer,
            Element::FoldTitleBackground(_) => PointerStyle::Pointer,
            Element::FoldContent(_) => PointerStyle::Default,
        }
    }

    pub fn block_id(&self) -> Option<crate::model::NodeId> {
        match self {
            Element::Line(e) => Some(e.block_id),
            Element::External(e) => Some(e.id),
            Element::HorizontalRule(e) => Some(e.node_id),
            Element::Blockquote(e) => Some(e.block_id),
            Element::CalloutBackground(e) => Some(e.node_id),
            Element::CalloutIcon(e) => Some(e.node_id),
            Element::ListMarker(_) => None,
            Element::FoldTitle(e) => Some(e.block_id),
            Element::FoldTitleBackground(_) => None,
            Element::FoldContent(_) => None,
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

    pub fn as_wrapper(&self) -> Option<&dyn crate::layout::elements::Wrapper> {
        match self {
            Element::CalloutBackground(e) => Some(e),
            Element::FoldContent(e) => Some(e),
            _ => None,
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
                    e.callout_type,
                    e.node_id,
                    split_edges,
                )))
            }
            Element::FoldContent(e) => Some(Element::FoldContent(FoldContentElement::new(
                Size::new(e.size.width, new_height),
                split_edges,
            ))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CalloutType, NodeId};

    fn all_wrapper_elements() -> Vec<Element> {
        vec![
            Element::CalloutBackground(CalloutBackgroundElement::new(
                Size::new(100.0, 100.0),
                CalloutType::Info,
                NodeId::new(),
                SplitEdges::default(),
            )),
            Element::FoldContent(FoldContentElement::new(
                Size::new(100.0, 100.0),
                SplitEdges::default(),
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
