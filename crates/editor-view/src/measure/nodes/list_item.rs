use std::borrow::Cow;

use editor_common::{EdgeInsets, Rect};
use editor_model::{ChildView, Node, NodeView};
use editor_resource::Resource;
use parley::setting::{FontFeature, Tag};
use parley::style::{FontFamily, FontFamilyName, FontFeatures, FontWeight, LineHeight, TextStyle};

use crate::glyph_run::{Glyph, GlyphRun, GraphemeSpan, Synthesis, TextDecoration};
use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;
use crate::measure::text::resolve::{ResolvedTextStyle, style_from_effective_modifiers};
use crate::measure::text::strut::compute_strut;
use crate::style::{Alignment, Decoration, DecorationData};

use super::dispatch::measure_child;
use super::line_geometry::{LineStrutExpansion, expand_first_line, first_line_info};
use crate::measure::Measurer;
use crate::measure::container::layout_padded;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredContent, MeasuredNode};

const MARKER_RECT_MIN_RATIO: f32 = 1.25;
const MARKER_OUTER_GAP_RATIO: f32 = 0.5;

fn list_item_max_font_size(node: &NodeView, base: &ResolvedTextStyle) -> f32 {
    let mut max = base.font_size;
    for desc in node.descendants() {
        if let ChildView::Leaf(l) = &desc
            && l.as_char().is_some()
        {
            let fs = style_from_effective_modifiers(
                &l.effective().values().cloned().collect::<Vec<_>>(),
            )
            .font_size;
            if fs > max {
                max = fs;
            }
        }
    }
    max
}

struct MarkerShape {
    glyph_runs: Vec<GlyphRun>,
}

fn shape_marker(
    node: &NodeView,
    style: &ResolvedTextStyle,
    resource: &mut Resource,
) -> Option<MarkerShape> {
    let parent = node.parent()?;
    let text = match parent.node() {
        Node::OrderedList(_) => format!("{}.", node.index().unwrap_or(0) + 1),
        _ => return None,
    };

    let font_id = resource.font_registry.intern(&style.font_family);
    let font_family_name = resource
        .font_registry
        .family_name_opt(font_id)
        .unwrap_or_default()
        .to_owned();

    let parley_style = TextStyle {
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(font_family_name))),
        font_size: style.font_size,
        line_height: LineHeight::FontSizeRelative(style.line_height),
        font_weight: FontWeight::new(style.font_weight as f32),
        font_features: FontFeatures::List(Cow::Borrowed(&[FontFeature {
            tag: Tag::new(b"tnum"),
            value: 1,
        }])),
        ..TextStyle::default()
    };

    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, &text, 1.0, false);
    let idx = builder.push_style(parley_style);
    builder.push_style_run(idx, 0..text.len());
    let mut layout = builder.build(&text);
    layout.break_all_lines(None);

    let mut glyph_runs = Vec::new();
    let line = layout.lines().next()?;
    for item in line.items() {
        if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
            let run = glyph_run.run();
            let run_x = glyph_run.offset();
            let mut glyph_x_advance = 0.0;
            let glyphs: Vec<Glyph> = glyph_run
                .glyphs()
                .map(|g| {
                    let gx = glyph_x_advance + g.x;
                    glyph_x_advance += g.advance;
                    Glyph {
                        id: g.id,
                        x: run_x + gx,
                        y: g.y,
                    }
                })
                .collect();

            let run_advance = glyph_run.advance();
            glyph_runs.push(GlyphRun {
                family_id: font_id,
                weight: style.font_weight,
                font_size: run.font_size(),
                synthesis: Synthesis::default(),
                color: String::new(),
                background_color: None,
                glyphs,
                decoration: TextDecoration::default(),
                offset_range: 0..0,
                link: None,
                text: text.clone(),
                x: 0.0,
                width: run_advance,
                graphemes: vec![GraphemeSpan {
                    advance: run_advance,
                    codepoints: text.chars().count() as u8,
                }],
                cursor_ascent: 0.0,
                cursor_descent: 0.0,
            });
        }
    }

    Some(MarkerShape { glyph_runs })
}

fn apply_baseline(mut glyph_runs: Vec<GlyphRun>, baseline: f32) -> DecorationData {
    for run in &mut glyph_runs {
        for g in &mut run.glyphs {
            g.y += baseline;
        }
    }
    DecorationData::Glyphs(glyph_runs)
}

pub(crate) fn measure_list_item(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let base_style =
        style_from_effective_modifiers(&node.effective().values().cloned().collect::<Vec<_>>());
    let marker_font_size = list_item_max_font_size(node, &base_style);
    let marker_style = ResolvedTextStyle {
        font_size: marker_font_size,
        ..base_style.clone()
    };

    let marker_strut = compute_strut(resource, &marker_style);
    let (marker_ascent, marker_descent) = marker_strut
        .as_ref()
        .map(|s| (s.ascent, s.descent))
        .unwrap_or((marker_font_size * 0.8, marker_font_size * 0.2));

    let marker_shape = shape_marker(node, &marker_style, resource);

    let measured_glyph_width = marker_shape
        .as_ref()
        .map(|s| s.glyph_runs.iter().map(|r| r.width).sum::<f32>())
        .unwrap_or(0.0);
    let marker_rect_width = measured_glyph_width.max(marker_font_size * MARKER_RECT_MIN_RATIO);
    let outer_gap = marker_font_size * MARKER_OUTER_GAP_RATIO;

    let padding = EdgeInsets {
        left: marker_rect_width + outer_gap,
        ..EdgeInsets::ZERO
    };

    let mut seam = |child, w, ctx: &MeasureContext, r: &mut Resource| {
        measure_child(measurer, child, w, ctx, r)
    };
    let measured = layout_padded(
        node,
        width,
        ctx,
        resource,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            alignment: Alignment::Start,
            page_break_policy: PageBreakPolicy::Auto,
        },
        &mut seam,
    );

    let expansion = LineStrutExpansion {
        ascent: marker_ascent,
        descent: marker_descent,
        min_line_height: marker_font_size * marker_style.line_height,
    };
    let (mut measured, line_top, line_height, line_baseline) =
        match expand_first_line(&measured, &expansion) {
            Some(e) => (e.tree, e.top, e.height, e.baseline),
            None => {
                let fallback_height = (marker_font_size * marker_style.line_height)
                    .max(marker_ascent + marker_descent);
                let fallback_baseline =
                    (fallback_height - (marker_ascent + marker_descent)).max(0.0) / 2.0
                        + marker_ascent;
                let top = first_line_info(&measured).map(|i| i.top).unwrap_or(0.0);
                (measured, top, fallback_height, fallback_baseline)
            }
        };

    let marker_data = match marker_shape {
        Some(shape) => apply_baseline(shape.glyph_runs, line_baseline),
        None => DecorationData::Bullet,
    };

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: 0.0,
                y: line_top,
                width: marker_rect_width,
                height: line_height,
            },
            data: marker_data,
        });
    }

    measured
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Anchor, Bias, DocLogs, DocView, Modifier, ModifierAttrLog, NodeAttrLog, NodeMarkerLog,
        NodeStyleLog, NodeType, SeqItem, SpanLog, SpanOp, StyleLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;

    use super::*;
    use crate::measure::types::MeasuredBox;

    fn resource_with_font() -> Resource {
        use fontique::ScriptExt;
        let mut resource = Resource::new_test();
        let font_data = include_bytes!("../../../assets/test-font.ttf");
        let families = resource.font_context.collection.register_fonts(
            fontique::Blob::new(Arc::new(font_data.to_vec())),
            Some(fontique::FontInfoOverride {
                family_name: Some("Noto Sans"),
                weight: Some(fontique::FontWeight::new(400.0)),
                ..Default::default()
            }),
        );
        let family_ids: Vec<_> = families.into_iter().map(|(id, _)| id).collect();
        for &script in fontique::Script::all_samples()
            .iter()
            .map(|(s, _)| s)
            .chain(&[
                fontique::Script::COMMON,
                fontique::Script::INHERITED,
                fontique::Script::UNKNOWN,
            ])
        {
            resource.font_context.collection.set_fallbacks(
                fontique::FallbackKey::new(script, None),
                family_ids.iter().copied(),
            );
        }
        resource
    }

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn ordered_list_doc_single() -> (DocLogs, Dot, Dot) {
        let root = Dot::ROOT;
        let ol = Dot::new(1, 1);
        let li = Dot::new(1, 2);
        let para = Dot::new(1, 3);
        let ch = Dot::new(1, 4);
        let para_root = Dot::new(1, 5);
        let items = vec![
            (
                ol,
                SeqItem::Block {
                    node_type: NodeType::OrderedList,
                    parents: vec![root],
                },
            ),
            (
                li,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, ol],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, ol, li],
                },
            ),
            (ch, SeqItem::Char('x')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        (logs(&items), li, ch)
    }

    fn bullet_list_doc_single() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let bl = Dot::new(1, 1);
        let li = Dot::new(1, 2);
        let para = Dot::new(1, 3);
        let para_root = Dot::new(1, 5);
        let items = vec![
            (
                bl,
                SeqItem::Block {
                    node_type: NodeType::BulletList,
                    parents: vec![root],
                },
            ),
            (
                li,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, bl],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bl, li],
                },
            ),
            (Dot::new(1, 4), SeqItem::Char('x')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        (logs(&items), li)
    }

    fn get_list_item<'a>(view: &'a DocView<'a>, li_dot: Dot) -> NodeView<'a> {
        view.node(li_dot).expect("list_item node")
    }

    fn default_style() -> ResolvedTextStyle {
        ResolvedTextStyle {
            font_family: String::new(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.6,
        }
    }

    #[test]
    fn ordered_marker_shapes_glyphs() {
        let (doc, li_dot, _ch_dot) = ordered_list_doc_single();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let li = get_list_item(&view, li_dot);
        let style = default_style();
        let mut res = resource_with_font();

        let shape = shape_marker(&li, &style, &mut res);
        assert!(
            shape.is_some(),
            "expected Some(MarkerShape) for ordered list"
        );
        let shape = shape.unwrap();
        assert!(
            !shape.glyph_runs.is_empty(),
            "expected non-empty glyph_runs"
        );
        let first_run = &shape.glyph_runs[0];
        assert_eq!(
            first_run.offset_range,
            0..0,
            "marker offset_range must be empty sentinel"
        );
        assert_eq!(first_run.text, "1.", "marker text must be '1.'");
    }

    #[test]
    fn bullet_returns_none() {
        let (doc, li_dot) = bullet_list_doc_single();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let li = get_list_item(&view, li_dot);
        let style = default_style();
        let mut res = resource_with_font();

        let shape = shape_marker(&li, &style, &mut res);
        assert!(shape.is_none(), "expected None for bullet list marker");
    }

    #[test]
    fn max_font_size_tracks_largest_char() {
        let root = Dot::ROOT;
        let ol = Dot::new(2, 1);
        let li = Dot::new(2, 2);
        let para = Dot::new(2, 3);
        let ch_small = Dot::new(2, 4);
        let ch_big = Dot::new(2, 5);
        let para_root = Dot::new(2, 6);
        let items = vec![
            (
                ol,
                SeqItem::Block {
                    node_type: NodeType::OrderedList,
                    parents: vec![root],
                },
            ),
            (
                li,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, ol],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, ol, li],
                },
            ),
            (ch_small, SeqItem::Char('a')),
            (ch_big, SeqItem::Char('B')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let mut doc = logs(&items);
        doc.spans = SpanLog::new()
            .apply(
                Dot::ROOT,
                SpanOp::AddSpan {
                    start: Anchor {
                        id: ch_big,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: ch_big,
                        bias: Bias::After,
                    },
                    modifier: Modifier::FontSize { value: 3200 },
                },
            )
            .unwrap();

        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let li_node = get_list_item(&view, li);
        let base = default_style();

        let max_fs = list_item_max_font_size(&li_node, &base);
        let expected = 3200.0 / 100.0 * (96.0 / 72.0);
        assert!(
            (max_fs - expected).abs() < 0.01,
            "expected max font size {expected}, got {max_fs}"
        );
        assert!(max_fs > base.font_size, "max font size must exceed base");

        let (plain_doc, plain_li_dot, _) = ordered_list_doc_single();
        let plain_pd = project_document(&plain_doc).unwrap();
        let plain_view = DocView::new(&plain_pd);
        let plain_li = get_list_item(&plain_view, plain_li_dot);
        let plain_max = list_item_max_font_size(&plain_li, &base);
        assert!(
            (plain_max - base.font_size).abs() < 0.01,
            "plain list item max font size should equal base: {plain_max}"
        );
    }

    use crate::measure::nodes::dispatch::measure_node;

    fn measure_root_of_list(resource: &mut Resource, is_ordered: bool) -> MeasuredNode {
        let root = Dot::ROOT;
        let list = Dot::new(10, 1);
        let li = Dot::new(10, 2);
        let para = Dot::new(10, 3);
        let ch = Dot::new(10, 4);
        let para_root = Dot::new(10, 5);
        let list_node_type = if is_ordered {
            NodeType::OrderedList
        } else {
            NodeType::BulletList
        };
        let items = vec![
            (
                list,
                SeqItem::Block {
                    node_type: list_node_type,
                    parents: vec![root],
                },
            ),
            (
                li,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, list],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, list, li],
                },
            ),
            (ch, SeqItem::Char('x')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        measure_node(
            &mut Measurer::new(),
            &root_node,
            400.0,
            &MeasureContext::default(),
            resource,
        )
    }

    fn extract_list_item_box(result: &MeasuredNode) -> &MeasuredBox {
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected Box at root");
        };
        let list_child = &root_box.children[0];
        let MeasuredContent::Box(ref list_box) = list_child.content else {
            panic!("expected Box at list");
        };
        let li_child = &list_box.children[0];
        let MeasuredContent::Box(ref li_box) = li_child.content else {
            panic!("expected Box at list_item");
        };
        li_box
    }

    #[test]
    fn ordered_list_item_has_glyph_marker() {
        let mut res = resource_with_font();
        let result = measure_root_of_list(&mut res, true);
        let li_box = extract_list_item_box(&result);
        assert_eq!(
            li_box.style.decorations.len(),
            1,
            "expected exactly 1 decoration"
        );
        let dec = &li_box.style.decorations[0];
        assert!(li_box.style.padding.left > 0.0, "padding.left must be > 0");
        match &dec.data {
            DecorationData::Glyphs(runs) => {
                assert!(!runs.is_empty(), "glyph runs must not be empty");
            }
            other => panic!("expected Glyphs decoration, got {:?}", other),
        }
    }

    #[test]
    fn bullet_list_item_has_bullet_marker() {
        let mut res = resource_with_font();
        let result = measure_root_of_list(&mut res, false);
        let li_box = extract_list_item_box(&result);
        assert_eq!(
            li_box.style.decorations.len(),
            1,
            "expected exactly 1 decoration"
        );
        let dec = &li_box.style.decorations[0];
        assert!(li_box.style.padding.left > 0.0, "padding.left must be > 0");
        assert!(
            matches!(dec.data, DecorationData::Bullet),
            "expected Bullet decoration"
        );
    }

    #[test]
    fn dispatch_wires_list_item() {
        let (doc, li_dot, _) = ordered_list_doc_single();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let li_node = get_list_item(&view, li_dot);
        let mut res = resource_with_font();

        let result = measure_node(
            &mut Measurer::new(),
            &li_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box at list_item");
        };
        assert_eq!(
            b.style.decorations.len(),
            1,
            "dispatch must route ListItem to measure_list_item (1 decoration)"
        );
    }

    #[test]
    fn marker_rect_geometry() {
        let mut res = resource_with_font();
        let result = measure_root_of_list(&mut res, true);
        let li_box = extract_list_item_box(&result);
        assert_eq!(li_box.style.decorations.len(), 1);
        let dec = &li_box.style.decorations[0];
        assert_eq!(dec.rect.x, 0.0, "marker rect x must be 0");
        assert!(
            dec.rect.width >= 16.0 * 1.25,
            "marker rect width must be >= base_font_size * MARKER_RECT_MIN_RATIO ({}), got {}",
            16.0 * 1.25,
            dec.rect.width
        );
        assert!(dec.rect.height > 0.0, "marker rect height must be > 0");
        assert!(dec.rect.y >= 0.0, "marker rect y must be >= 0");
    }

    #[test]
    fn large_descendant_font_grows_marker() {
        let root_s = Dot::ROOT;
        let ol_s = Dot::new(20, 1);
        let li_s = Dot::new(20, 2);
        let para_s = Dot::new(20, 3);
        let ch_s = Dot::new(20, 4);
        let para_root_s = Dot::new(20, 5);

        let root_l = Dot::ROOT;
        let ol_l = Dot::new(21, 1);
        let li_l = Dot::new(21, 2);
        let para_l = Dot::new(21, 3);
        let ch_l = Dot::new(21, 4);
        let para_root_l = Dot::new(21, 5);

        let make_items = |root: Dot, ol: Dot, li: Dot, para: Dot, ch: Dot, para_root: Dot| {
            vec![
                (
                    ol,
                    SeqItem::Block {
                        node_type: NodeType::OrderedList,
                        parents: vec![root],
                    },
                ),
                (
                    li,
                    SeqItem::Block {
                        node_type: NodeType::ListItem,
                        parents: vec![root, ol],
                    },
                ),
                (
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root, ol, li],
                    },
                ),
                (ch, SeqItem::Char('x')),
                (
                    para_root,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                    },
                ),
            ]
        };

        let items_s = make_items(root_s, ol_s, li_s, para_s, ch_s, para_root_s);
        let doc_s = logs(&items_s);

        let items_l = make_items(root_l, ol_l, li_l, para_l, ch_l, para_root_l);
        let mut doc_l = logs(&items_l);
        doc_l.spans = SpanLog::new()
            .apply(
                Dot::ROOT,
                SpanOp::AddSpan {
                    start: Anchor {
                        id: ch_l,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: ch_l,
                        bias: Bias::After,
                    },
                    modifier: Modifier::FontSize { value: 3200 },
                },
            )
            .unwrap();

        let mut res = resource_with_font();

        let pd_s = project_document(&doc_s).unwrap();
        let s = DocView::new(&pd_s);
        let li_s_node = get_list_item(&s, li_s);
        let result_s = measure_list_item(
            &mut Measurer::new(),
            &li_s_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b_s) = result_s.content else {
            panic!("expected Box for small list_item");
        };
        let dec_s = &b_s.style.decorations[0];

        let pd_l = project_document(&doc_l).unwrap();
        let l = DocView::new(&pd_l);
        let li_l_node = get_list_item(&l, li_l);
        let result_l = measure_list_item(
            &mut Measurer::new(),
            &li_l_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b_l) = result_l.content else {
            panic!("expected Box for large list_item");
        };
        let dec_l = &b_l.style.decorations[0];

        assert!(
            dec_l.rect.width > dec_s.rect.width,
            "large font marker width ({}) must exceed small font marker width ({})",
            dec_l.rect.width,
            dec_s.rect.width
        );
        assert!(
            dec_l.rect.height > dec_s.rect.height,
            "large font marker height ({}) must exceed small font marker height ({})",
            dec_l.rect.height,
            dec_s.rect.height
        );
    }

    #[test]
    fn ordinal_increments() {
        let root = Dot::ROOT;
        let ol = Dot::new(4, 1);
        let li1 = Dot::new(4, 2);
        let para1 = Dot::new(4, 3);
        let li2 = Dot::new(4, 5);
        let para2 = Dot::new(4, 6);
        let para_root = Dot::new(4, 8);
        let items = vec![
            (
                ol,
                SeqItem::Block {
                    node_type: NodeType::OrderedList,
                    parents: vec![root],
                },
            ),
            (
                li1,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, ol],
                },
            ),
            (
                para1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, ol, li1],
                },
            ),
            (Dot::new(4, 4), SeqItem::Char('a')),
            (
                li2,
                SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![root, ol],
                },
            ),
            (
                para2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, ol, li2],
                },
            ),
            (Dot::new(4, 7), SeqItem::Char('b')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let li1_node = get_list_item(&view, li1);
        let li2_node = get_list_item(&view, li2);
        let style = default_style();
        let mut res = resource_with_font();

        let shape1 = shape_marker(&li1_node, &style, &mut res).expect("first li marker");
        let shape2 = shape_marker(&li2_node, &style, &mut res).expect("second li marker");

        assert_eq!(
            shape1.glyph_runs[0].text, "1.",
            "first list item must be '1.'"
        );
        assert_eq!(
            shape2.glyph_runs[0].text, "2.",
            "second list item must be '2.'"
        );
    }
}
