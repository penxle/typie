mod builder;
mod codec;
mod utils;

pub use builder::DomSpec;
pub use codec::{HtmlContext, MarkHtmlCodec, MarkParseRule, NodeHtmlCodec, NodeParseRule};
pub use utils::{LengthUnit, parse_as, parse_font_size, parse_styles};

use builder::HtmlBuilder;
use codec::{
    collect_mark_parse_rules, collect_node_parse_rules, render_node_spec, try_parse_marks,
    try_parse_node,
};

use crate::model::*;
use crate::schema::Schema;
use anyhow::Result;
use scraper::{ElementRef, Html as HtmlDoc, Node as ScraperNode, Selector};

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
                if let Some(spec) = node.data().to_dom() {
                    render_node_spec(&spec, &ctx, id, &mut b);
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
        let mark_rules = collect_mark_parse_rules();

        let root = doc.root_element();
        parse_children(
            &root,
            None,
            None,
            &mut builder,
            &[],
            &schema,
            &node_rules,
            &mark_rules,
        )?;

        let mut fragment = builder.build();
        fragment.open_start = open_start;
        fragment.open_end = open_end;
        Ok(fragment.merge_adjacent_text_nodes())
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

fn parse_children(
    parent: &ElementRef,
    parent_id: Option<NodeId>,
    parent_type: Option<NodeType>,
    builder: &mut FragmentBuilder,
    marks: &[Mark],
    schema: &Schema,
    node_rules: &[NodeParseRule],
    mark_rules: &[MarkParseRule],
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
                    marks,
                    schema,
                    node_rules,
                    mark_rules,
                )?;
            }
            ScraperNode::Text(t) => {
                let s = t.text.to_string();
                if !s.is_empty() {
                    add_text(&s, parent_id, builder, marks.to_vec());
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
    marks: &[Mark],
    schema: &Schema,
    node_rules: &[NodeParseRule],
    mark_rules: &[MarkParseRule],
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
            let id = NodeId::new();
            let has_content = !schema.node_spec(node_type).content.is_leaf();
            *builder = std::mem::take(builder).add((id, FragmentNode::new(node, parent_id)));

            if has_content {
                parse_children(
                    elem,
                    Some(id),
                    Some(node_type),
                    builder,
                    &[],
                    schema,
                    node_rules,
                    mark_rules,
                )?;
            }

            return Ok(());
        }
    }

    let parsed = try_parse_marks(elem, mark_rules);
    let mut combined_marks = marks.to_vec();
    for mark in parsed.marks {
        if !combined_marks.iter().any(|m| m.as_type() == mark.as_type()) {
            combined_marks.push(mark);
        }
    }

    if let Some(content) = parsed.custom_content {
        if !content.is_empty() {
            add_text(&content, parent_id, builder, combined_marks);
        }
    } else {
        parse_children(
            elem,
            parent_id,
            parent_type,
            builder,
            &combined_marks,
            schema,
            node_rules,
            mark_rules,
        )?;
    }

    Ok(())
}

fn add_node(parent_id: Option<NodeId>, builder: &mut FragmentBuilder, node: Node) {
    let id = NodeId::new();
    *builder = std::mem::take(builder).add((id, FragmentNode::new(node, parent_id)));
}

fn add_text(
    content: &str,
    parent_id: Option<NodeId>,
    builder: &mut FragmentBuilder,
    marks: Vec<Mark>,
) {
    let text = Text::from(content);
    let len = text.char_len();
    for m in &marks {
        let _ = text.mark(0..len, m);
    }
    add_node(parent_id, builder, Node::Text(TextNode { text }));
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use crate::model::nodes::ParagraphNode;

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
        assert!(html.contains("<p>"));
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
            let segments = t.text.get_rich_text_segments();
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].0, "Red");
            assert_eq!(segments[1].0, "Blue");

            assert_eq!(segments[0].1.len(), 1);
            assert_eq!(segments[1].1.len(), 1);
        } else {
            panic!("Expected text node");
        }
    }
}
