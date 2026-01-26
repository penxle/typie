use crate::layout::elements::{FoldContentElement, FoldTitleBackgroundElement, SplitEdges};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const TITLE_PADDING_X: f32 = 12.0;
const TITLE_PADDING_Y: f32 = 8.0;
const CONTENT_PADDING_X: f32 = 24.0;
const CONTENT_PADDING_Y: f32 = 16.0;

pub const FOLD_BORDER_RADIUS: f32 = 8.0;
pub const FOLD_BORDER_WIDTH: f32 = 1.0;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FoldNode {}

impl NodeHtmlCodec for FoldNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("details").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("details", |_| {
            Some(Node::Fold(FoldNode {}))
        })]
    }
}

impl FoldNode {
    fn is_expanded(&self, ctx: &LayoutContext) -> bool {
        ctx.view_states
            .get(&ctx.node.node_id())
            .map(|s| s.fold_expanded())
            .unwrap_or(false)
    }

    fn layout_title(&self, ctx: &LayoutContext, inner_width: f32) -> Option<Rc<LayoutNode>> {
        let first_child = ctx.node.children().next()?;

        if !matches!(first_child.node(), Node::FoldTitle(_)) {
            return None;
        }

        let constraints = BoxConstraints::new(0.0, inner_width.max(0.0), 0.0, f32::MAX);
        Some(ctx.layout(&first_child, constraints))
    }

    fn layout_fold_content(&self, ctx: &LayoutContext, max_width: f32) -> Option<Rc<LayoutNode>> {
        let content_width = (max_width - CONTENT_PADDING_X * 2.0).max(0.0);
        let constraints = BoxConstraints::new(content_width, content_width, 0.0, f32::MAX);

        let fold_content = ctx.node.children().nth(1)?;
        if !matches!(fold_content.node(), Node::FoldContent(_)) {
            return None;
        }

        Some(ctx.layout(&fold_content, constraints))
    }

    fn build_child_nodes(
        &self,
        title_layout: Option<Rc<LayoutNode>>,
        content_layout: Option<Rc<LayoutNode>>,
        expanded: bool,
        max_width: f32,
    ) -> (Vec<PositionedNode>, f32, f32) {
        let mut children = Vec::new();
        let mut y = 0.0;

        let title_height = if let Some(title) = title_layout {
            let h = title.size.height;
            let full_title_h = TITLE_PADDING_Y + h + TITLE_PADDING_Y;

            children.push(PositionedNode {
                position: Point::new(TITLE_PADDING_X, TITLE_PADDING_Y),
                node: title,
            });
            y += full_title_h;
            full_title_h
        } else {
            0.0
        };

        if expanded {
            if let Some(content) = content_layout {
                let content_h = content.size.height;
                let wrapper_h = CONTENT_PADDING_Y + content_h + CONTENT_PADDING_Y;

                // fold의 bottom border가 잘리지 않고 다음 페이지로 넘어갈 수 있도록 함
                let trailing_padding = Rc::new(LayoutNode {
                    size: Size::new(max_width, CONTENT_PADDING_Y),
                    element: None,
                    children: None,
                    page_break_policy: Default::default(),
                    render_hints: Default::default(),
                    scope_id: None,
                });

                let wrapper = Rc::new(LayoutNode {
                    size: Size::new(max_width, wrapper_h),
                    element: Some(Element::FoldContent(FoldContentElement::new(
                        Size::new(max_width, wrapper_h),
                        SplitEdges::default(),
                    ))),
                    children: Some(vec![
                        PositionedNode {
                            position: Point::new(CONTENT_PADDING_X, CONTENT_PADDING_Y),
                            node: content,
                        },
                        PositionedNode {
                            position: Point::new(0.0, CONTENT_PADDING_Y + content_h),
                            node: trailing_padding,
                        },
                    ]),
                    page_break_policy: Default::default(),
                    render_hints: Default::default(),
                    scope_id: None,
                });

                children.push(PositionedNode {
                    position: Point::new(0.0, y),
                    node: wrapper,
                });

                y += wrapper_h;
            }
        }

        (children, y, title_height)
    }

    fn create_title_background_element(
        &self,
        ctx: &LayoutContext,
        total_width: f32,
        title_height: f32,
        expanded: bool,
    ) -> PositionedNode {
        let height = title_height;
        let size = Size::new(total_width, height);

        PositionedNode {
            position: Point::new(0.0, 0.0),
            node: Rc::new(LayoutNode {
                size,
                element: Some(Element::FoldTitleBackground(
                    FoldTitleBackgroundElement::new(size, expanded, ctx.node.node_id()),
                )),
                children: None,
                page_break_policy: PageBreakPolicy::Avoid,
                render_hints: Default::default(),
                scope_id: None,
            }),
        }
    }
}

impl Layout for FoldNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let expanded = self.is_expanded(ctx);
        let title_inner_width = constraints.max_width - TITLE_PADDING_X * 2.0;

        let title_layout = self.layout_title(ctx, title_inner_width);
        let content_layout = if expanded {
            self.layout_fold_content(ctx, constraints.max_width)
        } else {
            None
        };

        let (mut children, total_height, title_height) = self.build_child_nodes(
            title_layout,
            content_layout,
            expanded,
            constraints.max_width,
        );

        let total_size = Size::new(constraints.max_width, total_height);

        children.insert(
            0,
            self.create_title_background_element(
                ctx,
                constraints.max_width,
                title_height,
                expanded,
            ),
        );

        LayoutNode {
            size: total_size,
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}
