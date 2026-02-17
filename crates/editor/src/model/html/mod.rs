mod builder;
mod codec;
mod utils;

use crate::model::*;
use crate::schema::Schema;
use anyhow::Result;
pub use builder::DomSpec;
use builder::HtmlBuilder;
pub use codec::{
    AnnotationHtmlCodec, AnnotationParseRule, HtmlContext, NodeHtmlCodec, NodeParseRule,
    StyleHtmlCodec, StyleParseRule,
};
use codec::{
    collect_annotation_parse_rules, collect_node_parse_rules, collect_style_parse_rules,
    parse_inline_annotations, parse_inline_styles, render_node, try_parse_node,
};
use scraper::{ElementRef, Html as HtmlDoc, Node as ScraperNode, Selector};
use std::cell::Cell;
pub use utils::{LengthUnit, parse_as, parse_font_size, parse_styles};

impl Fragment {
    pub fn to_html(&self) -> String {
        let ctx = HtmlContext::new(self);
        let mut b = HtmlBuilder::new();

        b.open("meta")
            .attr("name", "typ-frag")
            .data("open-start", self.open_start())
            .data("open-end", self.open_end())
            .void();

        for id in self.top_level_node_ids() {
            if let Some(node) = self.node(id) {
                if let Some(spec) = ctx.node_to_dom(node.data()) {
                    render_node(&spec, &ctx, id, &mut b);
                }
            }
        }

        b.into_string()
    }

    pub fn from_html(html: &str) -> Result<Self> {
        let doc = HtmlDoc::parse_fragment(html);
        let mut builder = Fragment::builder();

        let (open_start, open_end) = parse_meta(&doc);

        let schema = Schema::default();
        let node_rules = collect_node_parse_rules();
        let style_rules = collect_style_parse_rules();
        let annotation_rules = collect_annotation_parse_rules();

        let root = doc.root_element();
        let pending_text_id = Cell::new(None);
        parse_children(
            &root,
            None,
            None,
            &mut builder,
            &[],
            &[],
            &schema,
            &node_rules,
            &style_rules,
            &annotation_rules,
            &pending_text_id,
        )?;

        let mut fragment = builder.build();
        fragment.open_start = open_start;
        fragment.open_end = open_end;
        Ok(fragment
            .normalize_font_weights()
            .merge_adjacent_text_nodes())
    }
}

fn parse_meta(doc: &HtmlDoc) -> (usize, usize) {
    let sel = Selector::parse(r#"meta[name="typ-frag"]"#).unwrap();
    doc.select(&sel)
        .next()
        .map(|m| {
            let os = m
                .value()
                .attr("data-open-start")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let oe = m
                .value()
                .attr("data-open-end")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            (os, oe)
        })
        .unwrap_or((0, 0))
}

fn read_node_id(elem: &ElementRef) -> Option<NodeId> {
    elem.value()
        .attr("data-node-id")
        .and_then(NodeId::from_string)
}

fn parse_children(
    parent: &ElementRef,
    parent_id: Option<NodeId>,
    parent_type: Option<NodeType>,
    builder: &mut FragmentBuilder,
    styles: &[Style],
    annotations: &[Annotation],
    schema: &Schema,
    node_rules: &[NodeParseRule],
    style_rules: &[StyleParseRule],
    annotation_rules: &[AnnotationParseRule],
    pending_text_id: &Cell<Option<NodeId>>,
) -> Result<()> {
    for child in parent.children() {
        match child.value() {
            ScraperNode::Element(_) => {
                let elem = ElementRef::wrap(child).unwrap();
                parse_element(
                    &elem,
                    parent_id,
                    parent_type,
                    builder,
                    styles,
                    annotations,
                    schema,
                    node_rules,
                    style_rules,
                    annotation_rules,
                    pending_text_id,
                )?;
            }
            ScraperNode::Text(t) => {
                let s = t.text.to_string();
                if !s.is_empty() {
                    let id = pending_text_id.take();
                    add_text(
                        &s,
                        parent_id,
                        builder,
                        styles.to_vec(),
                        annotations.to_vec(),
                        id,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_element(
    elem: &ElementRef,
    parent_id: Option<NodeId>,
    parent_type: Option<NodeType>,
    builder: &mut FragmentBuilder,
    styles: &[Style],
    annotations: &[Annotation],
    schema: &Schema,
    node_rules: &[NodeParseRule],
    style_rules: &[StyleParseRule],
    annotation_rules: &[AnnotationParseRule],
    pending_text_id: &Cell<Option<NodeId>>,
) -> Result<()> {
    let tag = elem.value().name();

    if tag == "meta" {
        return Ok(());
    }

    if let Some(node) = try_parse_node(elem, node_rules) {
        let node_type = node.as_type();
        let allowed = parent_type
            .map(|pt| schema.node_spec(pt).content.matches(node_type))
            .unwrap_or(true);

        if allowed {
            let id = read_node_id(elem).unwrap_or_else(NodeId::new);
            let has_content = !schema.node_spec(node_type).content.is_leaf();
            *builder = std::mem::take(builder).add((id, FragmentNode::new(node, parent_id)));

            if has_content {
                let child_text_id = Cell::new(None);
                parse_children(
                    elem,
                    Some(id),
                    Some(node_type),
                    builder,
                    &[],
                    &[],
                    schema,
                    node_rules,
                    style_rules,
                    annotation_rules,
                    &child_text_id,
                )?;
            }

            return Ok(());
        }
    }

    let parsed_styles = parse_inline_styles(elem, style_rules);
    let parsed_annotations = parse_inline_annotations(elem, annotation_rules);
    if parsed_styles.is_empty() && parsed_annotations.annotations.is_empty() {
        if let Some(id) = read_node_id(elem) {
            pending_text_id.set(Some(id));
        }
    }
    let mut combined_styles = styles.to_vec();
    for style in parsed_styles {
        if !combined_styles
            .iter()
            .any(|s| s.as_type() == style.as_type())
        {
            combined_styles.push(style);
        }
    }
    let mut combined_annotations = annotations.to_vec();
    for annotation in parsed_annotations.annotations {
        if !combined_annotations
            .iter()
            .any(|a| a.as_type() == annotation.as_type())
        {
            combined_annotations.push(annotation);
        }
    }

    if let Some(content) = parsed_annotations.custom_content {
        if !content.is_empty() {
            let id = pending_text_id.take();
            add_text(
                &content,
                parent_id,
                builder,
                combined_styles,
                combined_annotations,
                id,
            );
        }
    } else {
        parse_children(
            elem,
            parent_id,
            parent_type,
            builder,
            &combined_styles,
            &combined_annotations,
            schema,
            node_rules,
            style_rules,
            annotation_rules,
            pending_text_id,
        )?;
    }

    Ok(())
}

fn add_node(
    parent_id: Option<NodeId>,
    builder: &mut FragmentBuilder,
    node: Node,
    node_id: Option<NodeId>,
) {
    let id = node_id.unwrap_or_else(NodeId::new);
    *builder = std::mem::take(builder).add((id, FragmentNode::new(node, parent_id)));
}

fn add_text(
    content: &str,
    parent_id: Option<NodeId>,
    builder: &mut FragmentBuilder,
    styles: Vec<Style>,
    annotations: Vec<Annotation>,
    node_id: Option<NodeId>,
) {
    let text = Text::from(content);
    let len = text.char_len();
    for s in &styles {
        let _ = text.apply_style(0..len, s);
    }
    for ann in &annotations {
        let _ = text.apply_annotation(0..len, ann);
    }
    add_node(parent_id, builder, Node::Text(TextNode { text }), node_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::annotations::*;
    use crate::model::nodes::*;
    use crate::model::styles::*;

    #[test]
    fn test_roundtrip() {
        let text = Text::from("Hello");
        let id = NodeId::new();
        let frag = Fragment::builder()
            .add((id, FragmentNode::new(Node::Text(TextNode { text }), None)))
            .build();

        let html = frag.to_html();
        assert!(html.contains("Hello"));

        let parsed = Fragment::from_html(&html).unwrap();
        assert!(!parsed.is_empty());
    }

    #[test]
    fn test_paragraph_with_text() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Hello World"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        assert!(html.contains("<p "));
        assert!(html.contains("Hello World"));
        assert!(html.contains("</p>"));
    }

    #[test]
    fn test_meta_preserved() {
        let frag = Fragment {
            nodes: indexmap::IndexMap::new(),
            open_start: 2,
            open_end: 3,
        };
        let html = frag.to_html();

        assert!(html.contains(r#"data-open-start="2""#));
        assert!(html.contains(r#"data-open-end="3""#));

        let parsed = Fragment::from_html(&html).unwrap();
        assert_eq!(parsed.open_start, 2);
        assert_eq!(parsed.open_end, 3);
    }

    #[test]
    fn test_vscode_div_container_parses_as_multiple_paragraphs() {
        let html = r#"<div><div>Line 1</div><div>Line 2</div><div>Line 3</div></div>"#;

        let parsed = Fragment::from_html(html).unwrap();

        let top_levels = parsed.top_level_node_ids();
        assert_eq!(
            top_levels.len(),
            3,
            "Expected 3 paragraphs, got {}",
            top_levels.len()
        );

        for id in &top_levels {
            let node = parsed.node(*id).unwrap();
            assert!(
                matches!(node.data(), Node::Paragraph(_)),
                "Expected Paragraph node"
            );
        }
    }

    #[test]
    fn test_colored_spans_merged_into_single_text_node() {
        let html = r#"<p><span style="color: rgb(255, 0, 0);">Red</span><span style="color: rgb(0, 0, 255);">Blue</span></p>"#;
        let parsed = Fragment::from_html(html).unwrap();

        let top_levels = parsed.top_level_node_ids();
        assert_eq!(top_levels.len(), 1);

        let para_id = top_levels[0];
        let children = parsed.children_of_node(para_id);

        assert_eq!(
            children.len(),
            1,
            "Expected 1 merged text node, but got {}",
            children.len()
        );

        let (_text_id, text_node) = children[0];
        if let Node::Text(t) = text_node.data() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].text, "Red");
            assert_eq!(segments[1].text, "Blue");

            assert!(
                segments[0]
                    .styles
                    .iter()
                    .any(|s| matches!(s, Style::TextColor(tc) if tc.color == "red"))
            );
            assert!(
                segments[1]
                    .styles
                    .iter()
                    .any(|s| matches!(s, Style::TextColor(tc) if tc.color == "indigo"))
            );
        } else {
            panic!("Expected text node");
        }
    }

    fn assert_node_type(frag: &Fragment, id: NodeId, expected: &str) {
        let node = frag.node(id).unwrap();
        let actual = format!("{:?}", std::mem::discriminant(node.data()));
        assert!(
            match expected {
                "Paragraph" => matches!(node.data(), Node::Paragraph(_)),
                "Blockquote" => matches!(node.data(), Node::Blockquote(_)),
                "BulletList" => matches!(node.data(), Node::BulletList(_)),
                "OrderedList" => matches!(node.data(), Node::OrderedList(_)),
                "ListItem" => matches!(node.data(), Node::ListItem(_)),
                "Image" => matches!(node.data(), Node::Image(_)),
                "Embed" => matches!(node.data(), Node::Embed(_)),
                "Archived" => matches!(node.data(), Node::Archived(_)),
                "File" => matches!(node.data(), Node::File(_)),
                "HorizontalRule" => matches!(node.data(), Node::HorizontalRule(_)),
                "HardBreak" => matches!(node.data(), Node::HardBreak(_)),
                "PageBreak" => matches!(node.data(), Node::PageBreak(_)),
                "Callout" => matches!(node.data(), Node::Callout(_)),
                "Fold" => matches!(node.data(), Node::Fold(_)),
                "FoldTitle" => matches!(node.data(), Node::FoldTitle(_)),
                "FoldContent" => matches!(node.data(), Node::FoldContent(_)),
                "Table" => matches!(node.data(), Node::Table(_)),
                "TableRow" => matches!(node.data(), Node::TableRow(_)),
                "TableCell" => matches!(node.data(), Node::TableCell(_)),
                "Text" => matches!(node.data(), Node::Text(_)),
                _ => false,
            },
            "Expected {expected}, got {actual}"
        );
    }

    #[test]
    fn test_roundtrip_paragraph_attrs() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(
                    Node::Paragraph(ParagraphNode {
                        align: TextAlign::Center,
                        line_height: 2.0,
                    }),
                    None,
                ),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Centered"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);

        if let Node::Paragraph(p) = parsed.node(top[0]).unwrap().data() {
            assert_eq!(p.align, TextAlign::Center);
            assert!((p.line_height - 2.0).abs() < 0.01);
        } else {
            panic!("Expected Paragraph");
        }
    }

    #[test]
    fn test_roundtrip_blockquote() {
        let bq_id = NodeId::new();
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let frag = Fragment::builder()
            .add((
                bq_id,
                FragmentNode::new(
                    Node::Blockquote(BlockquoteNode {
                        variant: BlockquoteVariant::LeftQuote,
                    }),
                    None,
                ),
            ))
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), Some(bq_id)),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Quoted"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);
        assert_node_type(&parsed, top[0], "Blockquote");

        if let Node::Blockquote(bq) = parsed.node(top[0]).unwrap().data() {
            assert_eq!(bq.variant, BlockquoteVariant::LeftQuote);
        }
    }

    #[test]
    fn test_roundtrip_callout() {
        let callout_id = NodeId::new();
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let frag = Fragment::builder()
            .add((
                callout_id,
                FragmentNode::new(
                    Node::Callout(CalloutNode {
                        variant: CalloutVariant::Info,
                    }),
                    None,
                ),
            ))
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), Some(callout_id)),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Info"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);
        assert_node_type(&parsed, top[0], "Callout");
    }

    #[test]
    fn test_roundtrip_table() {
        let table_id = NodeId::new();
        let row_id = NodeId::new();
        let cell_id = NodeId::new();
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let frag = Fragment::builder()
            .add((
                table_id,
                FragmentNode::new(
                    Node::Table(TableNode {
                        border_style: TableBorderStyle::Solid,
                        align: TableAlign::Center,
                        proportion: 0.75,
                    }),
                    None,
                ),
            ))
            .add((
                row_id,
                FragmentNode::new(Node::TableRow(TableRowNode {}), Some(table_id)),
            ))
            .add((
                cell_id,
                FragmentNode::new(
                    Node::TableCell(TableCellNode {
                        col_width: Some(0.5),
                    }),
                    Some(row_id),
                ),
            ))
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), Some(cell_id)),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Cell"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);
        assert_node_type(&parsed, top[0], "Table");

        if let Node::Table(t) = parsed.node(top[0]).unwrap().data() {
            assert_eq!(t.border_style, TableBorderStyle::Solid);
            assert_eq!(t.align, TableAlign::Center);
            assert_eq!(t.proportion, 0.75);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_roundtrip_file() {
        let file_id = NodeId::new();
        let frag = Fragment::builder()
            .add((
                file_id,
                FragmentNode::new(
                    Node::File(FileNode {
                        id: Some("file-123".to_string()),
                        upload_id: None,
                    }),
                    None,
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);
        assert_node_type(&parsed, top[0], "File");

        if let Node::File(f) = parsed.node(top[0]).unwrap().data() {
            assert_eq!(f.id.as_deref(), Some("file-123"));
        } else {
            panic!("Expected File");
        }
    }

    #[test]
    fn test_roundtrip_styles() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let text = Text::from("BoldItalic");
        let _ = text.apply_style(0..4, &Style::FontWeight(FontWeightStyle { weight: 700 }));
        let _ = text.apply_style(4..10, &Style::Italic(ItalicStyle {}));

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(Node::Text(TextNode { text }), Some(para_id)),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        let children = parsed.children_of_node(top[0]);
        assert_eq!(children.len(), 1);

        if let Node::Text(t) = children[0].1.data() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].text, "Bold");
            assert!(
                segments[0]
                    .styles
                    .iter()
                    .any(|s| matches!(s, Style::FontWeight(_)))
            );
            assert_eq!(segments[1].text, "Italic");
            assert!(
                segments[1]
                    .styles
                    .iter()
                    .any(|s| matches!(s, Style::Italic(_)))
            );
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_roundtrip_font_weight_precision() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let text = Text::from("W800");
        let _ = text.apply_style(0..4, &Style::FontWeight(FontWeightStyle { weight: 800 }));

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(Node::Text(TextNode { text }), Some(para_id)),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        let children = parsed.children_of_node(top[0]);
        if let Node::Text(t) = children[0].1.data() {
            let segments = t.text.get_segments();
            if let Some(Style::FontWeight(fw)) = segments[0].styles.first() {
                assert_eq!(fw.weight, 800, "FontWeight 800 lost: got {}", fw.weight);
            }
        }
    }

    #[test]
    fn test_roundtrip_node_ids_preserved() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Keep my ID"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0], para_id);

        let children = parsed.children_of_node(para_id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, text_id);
    }

    #[test]
    fn test_roundtrip_node_ids_complex() {
        let bq_id = NodeId::new();
        let p1_id = NodeId::new();
        let t1_id = NodeId::new();
        let p2_id = NodeId::new();
        let t2_id = NodeId::new();

        let text2 = Text::from("Linked");

        let frag = Fragment::builder()
            .add((
                bq_id,
                FragmentNode::new(Node::Blockquote(BlockquoteNode::default()), None),
            ))
            .add((
                p1_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), Some(bq_id)),
            ))
            .add((
                t1_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Hello"),
                    }),
                    Some(p1_id),
                ),
            ))
            .add((
                p2_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), Some(bq_id)),
            ))
            .add((
                t2_id,
                FragmentNode::new(Node::Text(TextNode { text: text2 }), Some(p2_id)),
            ))
            .build();

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();

        assert_eq!(parsed.top_level_node_ids(), vec![bq_id]);

        let bq_children = parsed.children_of_node(bq_id);
        assert_eq!(bq_children.len(), 2);
        assert_eq!(bq_children[0].0, p1_id);
        assert_eq!(bq_children[1].0, p2_id);

        let t1_children = parsed.children_of_node(p1_id);
        assert_eq!(t1_children.len(), 1);
        assert_eq!(t1_children[0].0, t1_id);

        let t2_children = parsed.children_of_node(p2_id);
        assert_eq!(t2_children.len(), 1);
        assert_eq!(t2_children[0].0, t2_id);
    }

    #[test]
    fn test_roundtrip_open_fragment() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();
        let mut frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(
                    Node::Text(TextNode {
                        text: Text::from("Open"),
                    }),
                    Some(para_id),
                ),
            ))
            .build();
        frag.open_start = 1;
        frag.open_end = 1;

        let html = frag.to_html();
        let parsed = Fragment::from_html(&html).unwrap();
        assert_eq!(parsed.open_start, 1);
        assert_eq!(parsed.open_end, 1);
    }

    #[test]
    fn test_image_paste() {
        let html = r#"<img data-image-id="test-id" data-proportion="1.5">"#;
        let parsed = Fragment::from_html(html).unwrap();

        let top_levels = parsed.top_level_node_ids();
        assert_eq!(top_levels.len(), 1);

        let img_id = top_levels[0];
        let img_node = parsed.node(img_id).unwrap();

        if let Node::Image(img) = img_node.data() {
            assert_eq!(img.id.as_deref(), Some("test-id"));
            assert_eq!(img.proportion, 1.5);
        } else {
            panic!("Expected Image node");
        }
    }

    #[test]
    fn test_roundtrip_link_annotation() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let text = Text::from("Click here");
        let _ = text.apply_annotation(
            0..10,
            &Annotation::Link(LinkAnnotation {
                href: "https://example.com".into(),
            }),
        );

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(Node::Text(TextNode { text }), Some(para_id)),
            ))
            .build();

        let html = frag.to_html();
        assert!(html.contains("<a "));
        assert!(html.contains("https://example.com"));
        assert!(html.contains("Click here"));

        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        assert_eq!(top.len(), 1);

        let children = parsed.children_of_node(top[0]);
        assert_eq!(children.len(), 1);

        if let Node::Text(t) = children[0].1.data() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].text, "Click here");
            assert_eq!(segments[0].annotations.len(), 1);

            assert!(matches!(
                &segments[0].annotations[0],
                Annotation::Link(link) if link.href == "https://example.com"
            ));
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_roundtrip_ruby_annotation() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let text = Text::from("漢字");
        let _ = text.apply_annotation(
            0..2,
            &Annotation::Ruby(RubyAnnotation {
                text: "かんじ".into(),
            }),
        );

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(Node::Text(TextNode { text }), Some(para_id)),
            ))
            .build();

        let html = frag.to_html();
        assert!(html.contains("<ruby>"));
        assert!(html.contains("<rt>"));
        assert!(html.contains("かんじ"));

        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        let children = parsed.children_of_node(top[0]);

        if let Node::Text(t) = children[0].1.data() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].text, "漢字");
            assert_eq!(segments[0].annotations.len(), 1);

            assert!(matches!(
                &segments[0].annotations[0],
                Annotation::Ruby(ruby) if ruby.text == "かんじ"
            ));
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_roundtrip_link_with_styles() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let text = Text::from("Bold link");
        let _ = text.apply_style(0..9, &Style::FontWeight(FontWeightStyle { weight: 700 }));
        let _ = text.apply_annotation(
            0..9,
            &Annotation::Link(LinkAnnotation {
                href: "https://example.com".into(),
            }),
        );

        let frag = Fragment::builder()
            .add((
                para_id,
                FragmentNode::new(Node::Paragraph(ParagraphNode::default()), None),
            ))
            .add((
                text_id,
                FragmentNode::new(Node::Text(TextNode { text }), Some(para_id)),
            ))
            .build();

        let html = frag.to_html();

        let parsed = Fragment::from_html(&html).unwrap();

        let top = parsed.top_level_node_ids();
        let children = parsed.children_of_node(top[0]);

        if let Node::Text(t) = children[0].1.data() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].text, "Bold link");

            assert!(
                segments[0]
                    .styles
                    .iter()
                    .any(|s| matches!(s, Style::FontWeight(_)))
            );
            assert_eq!(segments[0].annotations.len(), 1);

            assert!(matches!(
                &segments[0].annotations[0],
                Annotation::Link(link) if link.href == "https://example.com"
            ));
        } else {
            panic!("Expected Text node");
        }
    }
}
