use crate::global::{GLOBALS, TextBrush};
use crate::layout::elements::{FoldTitleIconElement, LineElement, build_metrics};
use crate::layout::{
    Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode, measure_strut,
};
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
    1.6 // fold_title uses raw f32 for parley (not stored in document)
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
            .filter_map(|child| match child.node()? {
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

        let (cascade_family, _, _) = ctx.resolve_cascade_font();

        let layout = GLOBALS.with(|globals| {
            let globals = globals.borrow();

            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let setup_defaults = |builder: &mut parley::RangedBuilder<'_, TextBrush>| {
                builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
                    FontFamilyName::Named(cascade_family.clone().into()),
                )));
                builder.push_default(StyleProperty::FontSize(14.0));
                builder.push_default(StyleProperty::FontWeight(FontWeight::new(
                    FOLD_TITLE_FONT_WEIGHT as f32,
                )));
                builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                    line_height,
                )));
                builder.push_default(StyleProperty::LetterSpacing(0.0));
                builder.push_default(StyleProperty::Brush(TextBrush {
                    color: "ui.text.faint".to_string(),
                    ..Default::default()
                }));

                builder.push_default(StyleProperty::FontFeatures(FontFeatures::Source(
                    Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
                )));
            };

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
            builder.push_default(StyleProperty::WordBreak(WordBreak::BreakAll));

            setup_defaults(&mut builder);

            // Mapping-based font family resolution
            {
                let font_mappings = globals.font_mappings.borrow();
                let font_interner = globals.font_family_interner.borrow();

                let interned_primary = font_interner
                    .get(cascade_family.as_str())
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::from(cascade_family.as_str()));

                let primary_weight = FOLD_TITLE_FONT_WEIGHT;
                let mut current_resolved: Option<(std::sync::Arc<str>, u16)> = None;
                let mut range_start_byte = 0usize;

                for (byte_idx, ch) in text.char_indices() {
                    let cp = ch as u32;
                    let resolved = font_mappings
                        .get(&(interned_primary.clone(), primary_weight, cp))
                        .cloned()
                        .unwrap_or_else(|| (interned_primary.clone(), primary_weight));

                    let is_same = current_resolved
                        .as_ref()
                        .map_or(false, |prev| prev.0 == resolved.0 && prev.1 == resolved.1);

                    if !is_same {
                        if let Some(ref prev) = current_resolved {
                            let byte_range = range_start_byte..byte_idx;
                            builder.push(
                                StyleProperty::FontFamily(FontFamily::Single(
                                    FontFamilyName::Named(prev.0.to_string().into()),
                                )),
                                byte_range.clone(),
                            );
                            if prev.1 != primary_weight {
                                builder.push(
                                    StyleProperty::FontWeight(FontWeight::new(prev.1 as f32)),
                                    byte_range,
                                );
                            }
                        }
                        current_resolved = Some(resolved);
                        range_start_byte = byte_idx;
                    }
                }

                if let Some(ref prev) = current_resolved {
                    let byte_range = range_start_byte..text.len();
                    builder.push(
                        StyleProperty::FontFamily(FontFamily::Single(FontFamilyName::Named(
                            prev.0.to_string().into(),
                        ))),
                        byte_range.clone(),
                    );
                    if prev.1 != primary_weight {
                        builder.push(
                            StyleProperty::FontWeight(FontWeight::new(prev.1 as f32)),
                            byte_range,
                        );
                    }
                }
            }

            let mut layout = builder.build(&text);
            layout.break_all_lines(Some(content_width));
            layout.align(
                Some(content_width),
                parley::Alignment::Left,
                parley::AlignmentOptions::default(),
            );

            (
                layout,
                measure_strut(
                    &mut lcx,
                    &mut fcx,
                    &cascade_family,
                    FOLD_TITLE_FONT_WEIGHT,
                    14.0,
                    line_height,
                ),
            )
        });

        let (layout, strut_metrics) = layout;
        let layout = Rc::new(layout);
        let metrics = build_metrics(
            &layout,
            &text,
            ctx.scale_factor,
            strut_metrics,
            None,
            line_height,
        );

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

        let toggle_element = FoldTitleIconElement::new(
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
                    element: Some(Element::FoldTitleIcon(toggle_element)),
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
