use crate::global::GLOBALS;
use crate::layout::elements::{FoldTitleElement, LineElement, build_metrics};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::model::{Node, PreeditDecor};
use crate::types::{BoxConstraints, Point, Size};
use crate::utils::char_to_byte_offset;
use macros::Codec;
use parley::style::*;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::rc::Rc;

const TOGGLE_ICON_WIDTH: f32 = 20.0;
const TOGGLE_ICON_PADDING: f32 = 8.0;
const CONTENT_OFFSET: f32 = TOGGLE_ICON_WIDTH + TOGGLE_ICON_PADDING;
pub const FOLD_TITLE_FONT_WEIGHT: u16 = 500;

fn default_line_height() -> f32 {
    1.6
}

fn preedit_for_node<'a>(ctx: &'a LayoutContext<'a>) -> Option<&'a PreeditDecor> {
    ctx.decorations
        .preedit
        .as_ref()
        .filter(|preedit| preedit.node_id == ctx.node.node_id())
}

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FoldTitleNode {}

impl NodeHtmlCodec for FoldTitleNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("summary").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("summary", |_| {
            Some(Node::FoldTitle(FoldTitleNode {}))
        })]
    }
}

impl Layout for FoldTitleNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let mut text = ctx
            .node
            .children()
            .filter_map(|child| match child.node() {
                Node::Text(node) => Some(node.text.to_string()),
                _ => None,
            })
            .collect::<String>();

        let preedit = preedit_for_node(ctx);

        if let Some(preedit) = preedit {
            let idx = char_to_byte_offset(&text, preedit.offset);
            text.insert_str(idx, &preedit.text);
        }

        let is_text_empty = text.is_empty();
        if text.is_empty() {
            text = "\u{200B}".to_string();
        }

        let line_height = default_line_height();
        let content_width = (constraints.max_width - CONTENT_OFFSET).max(0.0);

        let layout = GLOBALS.with(|globals| {
            let globals = globals.borrow();

            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let setup_defaults = |builder: &mut parley::RangedBuilder<'_, String>| {
                builder.push_default(StyleProperty::FontStack(FontStack::Single(
                    FontFamily::Named(ctx.default_styles.font_family().into()),
                )));
                builder.push_default(StyleProperty::FontSize(14.0));
                builder.push_default(StyleProperty::FontWeight(FontWeight::new(
                    FOLD_TITLE_FONT_WEIGHT as f32,
                )));
                builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                    line_height,
                )));
                builder.push_default(StyleProperty::LetterSpacing(0.0));
                builder.push_default(StyleProperty::Brush("ui.text.subtle".to_string()));

                builder.push_default(StyleProperty::FontFeatures(FontSettings::Source(
                    Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
                )));
            };

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
            builder.push_default(StyleProperty::WordBreak(WordBreakStrength::BreakAll));

            setup_defaults(&mut builder);

            let mut layout = builder.build(&text);
            layout.break_all_lines(Some(content_width));
            layout.align(
                Some(content_width),
                parley::Alignment::Left,
                parley::AlignmentOptions::default(),
            );

            let mut dummy_builder = lcx.ranged_builder(&mut fcx, "\u{200B}", 1.0, false);
            setup_defaults(&mut dummy_builder);

            let mut dummy_layout = dummy_builder.build("\u{200B}");
            dummy_layout.break_all_lines(None);
            let dummy_line = dummy_layout.lines().next().unwrap();
            let dummy_metrics = dummy_line.metrics();
            let default_height = dummy_metrics.ascent + dummy_metrics.descent;

            (layout, default_height)
        });

        let (layout, default_height) = layout;
        let layout = Rc::new(layout);
        let metrics = build_metrics(&layout, &text, ctx.scale_factor, default_height);

        let expanded = ctx
            .view_states
            .get(
                &ctx.node
                    .parent()
                    .map(|p| p.node_id())
                    .unwrap_or(ctx.node.node_id()),
            )
            .map(|s| s.fold_expanded())
            .unwrap_or(false);

        let fold_id = ctx
            .node
            .parent()
            .map(|p| p.node_id())
            .unwrap_or(ctx.node.node_id());

        let mut children = Vec::new();
        let mut y_offset = 0.0;

        let text_rc: Rc<str> = Rc::from(text);
        let preedit = preedit_for_node(ctx).cloned();
        for (line_idx, metric) in metrics.iter().enumerate() {
            let line_element = LineElement::build(
                ctx.node.node_id(),
                Size::new(content_width, metric.height + metric.leading),
                line_idx,
                layout.clone(),
                metric.clone(),
                preedit.clone(),
                is_text_empty,
                text_rc.clone(),
                Vec::new(),
                Vec::new(),
                false,
            );

            children.push(PositionedNode {
                position: Point::new(CONTENT_OFFSET, y_offset),
                node: Rc::new(LayoutNode {
                    size: line_element.size,
                    element: Some(Element::Line(line_element)),
                    children: None,
                    page_break_policy: PageBreakPolicy::Avoid,
                    render_hints: Default::default(),
                    scope_id: None,
                }),
            });

            y_offset += metric.height + metric.leading;
        }

        let total_size = Size::new(constraints.max_width, y_offset);

        let toggle_element = FoldTitleElement::new(
            Size::new(TOGGLE_ICON_WIDTH, y_offset),
            ctx.node.node_id(),
            fold_id,
            expanded,
        );

        children.insert(
            0,
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(TOGGLE_ICON_WIDTH, y_offset),
                    element: Some(Element::FoldTitle(toggle_element)),
                    children: None,
                    page_break_policy: PageBreakPolicy::Avoid,
                    render_hints: Default::default(),
                    scope_id: None,
                }),
            },
        );

        LayoutNode {
            size: total_size,
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}
