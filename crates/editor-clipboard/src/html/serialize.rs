use super::markup::{
    HtmlTarget, NodeMarkup, markup_for_node, write_clipboard_text, write_close_tag, write_open_tag,
};
use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{Fragment, Modifier, PlainNode};
use editor_resource::Resource;
use serde::Serialize;

pub(crate) fn serialize_clipboard_slice(
    slice: &Slice,
    resource: &Resource,
    assets: &[crate::ClipboardAsset],
) -> String {
    let mut out = String::new();
    out.push_str(r#"<meta charset="utf-8">"#);
    let mut meta_json = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut meta_json);
    slice
        .serialize(serde_stacker::Serializer::new(&mut serializer))
        .expect("Slice serde");
    let meta_b64 = STANDARD.encode(meta_json);
    out.push_str(&format!(
        r#"<meta data-slice-v2="{meta_b64}" data-version="1">"#,
    ));
    out.push_str("<div data-root>");
    serialize_forest(&slice.content, resource, assets, &mut out);
    out.push_str("</div>");
    out
}

enum SerializeTask<'a> {
    Nodes(&'a [Fragment]),
    Close(&'static str),
}

fn serialize_forest(
    fragments: &[Fragment],
    resource: &Resource,
    assets: &[crate::ClipboardAsset],
    out: &mut String,
) {
    let mut tasks = vec![SerializeTask::Nodes(fragments)];
    while let Some(task) = tasks.pop() {
        let fragment = match task {
            SerializeTask::Nodes(fragments) => {
                let Some((fragment, rest)) = fragments.split_first() else {
                    continue;
                };
                if let Some(annotation) = ruby_annotation(fragment) {
                    let count = fragments
                        .iter()
                        .take_while(|fragment| ruby_annotation(fragment) == Some(annotation))
                        .count();
                    out.push_str("<ruby>");
                    for fragment in &fragments[..count] {
                        let PlainNode::Text(text) = &fragment.node else {
                            unreachable!()
                        };
                        write_clipboard_text(&text.text, &fragment.modifiers, resource, out);
                    }
                    out.push_str("<rt>");
                    write_clipboard_text(annotation, &[], resource, out);
                    out.push_str("</rt></ruby>");
                    tasks.push(SerializeTask::Nodes(&fragments[count..]));
                    continue;
                }
                tasks.push(SerializeTask::Nodes(rest));
                fragment
            }
            SerializeTask::Close(tag) => {
                write_close_tag(out, tag);
                continue;
            }
        };
        let asset_id = match &fragment.node {
            PlainNode::Image(node) => node.id.as_deref(),
            PlainNode::Embed(node) => node.id.as_deref(),
            PlainNode::File(node) => node.id.as_deref(),
            _ => None,
        };
        let asset = asset_id.and_then(|id| assets.iter().find(|asset| asset.id == id));
        match markup_for_node(&fragment.node, HtmlTarget::Clipboard) {
            NodeMarkup::Text => {
                let PlainNode::Text(text) = &fragment.node else {
                    unreachable!()
                };
                write_clipboard_text(&text.text, &fragment.modifiers, resource, out);
            }
            NodeMarkup::Tab => out.push('\t'),
            NodeMarkup::Children => tasks.push(SerializeTask::Nodes(&fragment.children)),
            NodeMarkup::Skip => {}
            NodeMarkup::Element { tag, attrs, void } => {
                let mut extra = Vec::new();
                if let Some(asset) = asset {
                    extra.push((
                        if tag == "img" { "src" } else { "href" },
                        Some(asset.url.as_str()),
                    ));
                }
                if tag == "img" {
                    extra.push((
                        "alt",
                        Some(asset.map_or("이미지", |asset| asset.label.as_str())),
                    ));
                }
                // Keep copied fold contents visible in external HTML.
                if tag == "details"
                    && fragment
                        .children
                        .iter()
                        .any(|child| matches!(child.node, PlainNode::FoldContent(_)))
                {
                    extra.push(("open", None));
                }
                write_open_tag(out, tag, &attrs, &extra);
                if !void {
                    tasks.push(SerializeTask::Close(tag));
                    if matches!(fragment.node, PlainNode::Embed(_) | PlainNode::File(_)) {
                        let label = asset.map_or(
                            if matches!(fragment.node, PlainNode::Embed(_)) {
                                "임베드"
                            } else {
                                "파일"
                            },
                            |asset| asset.label.as_str(),
                        );
                        write_clipboard_text(label, &[], resource, out);
                    }
                    tasks.push(SerializeTask::Nodes(&fragment.children));
                }
            }
        }
    }
}

// Ruby groups are contiguous base text with the same annotation, even when
// other modifiers differ. Atoms and block boundaries end the group.
fn ruby_annotation(fragment: &Fragment) -> Option<&str> {
    if !matches!(fragment.node, PlainNode::Text(_)) {
        return None;
    }
    fragment
        .modifiers
        .iter()
        .find_map(|modifier| match modifier {
            Modifier::Ruby { text } if !text.is_empty() => Some(text.as_str()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::Slice;
    use crate::test_doc::DocBuilder;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        Fragment, Modifier, NodeType, PlainNode, PlainParagraphNode, PlainTextNode,
    };
    use editor_state::{Position, Selection};

    #[test]
    fn copy_assets_have_external_urls_and_preserve_internal_references() {
        use crate::{ClipboardAsset, PayloadSource};
        use editor_model::{PlainEmbedNode, PlainFileNode, PlainImageNode};
        let slice = Slice::new(
            vec![
                Fragment::leaf(PlainNode::Image(PlainImageNode {
                    id: Some("image".into()),
                    ..Default::default()
                })),
                Fragment::leaf(PlainNode::File(PlainFileNode {
                    id: Some("file".into()),
                })),
                Fragment::leaf(PlainNode::Embed(PlainEmbedNode {
                    id: Some("embed".into()),
                })),
            ],
            0,
            0,
        );
        let assets = [
            ClipboardAsset {
                id: "image".into(),
                url: "https://typie.net/original-images/a.png?x=1&y=2".into(),
                label: "이미지".into(),
            },
            ClipboardAsset {
                id: "file".into(),
                url: "https://typie.net/files/a.pdf".into(),
                label: "<notes>.pdf".into(),
            },
            ClipboardAsset {
                id: "embed".into(),
                url: "https://example.com/article".into(),
                label: "Article".into(),
            },
        ];
        let resource = Resource::new_test();
        let payload = slice.to_payload(&resource, &assets);
        assert!(
            payload
                .html
                .contains(r#"src="https://typie.net/original-images/a.png?x=1&amp;y=2""#)
        );
        assert!(
            payload
                .html
                .contains(r#"href="https://typie.net/files/a.pdf">&lt;notes&gt;.pdf</a>"#)
        );
        assert!(
            payload
                .html
                .contains(r#"href="https://example.com/article">Article</a>"#)
        );
        assert!(!payload.html.contains("iframe"));
        assert!(!payload.html.contains("data:image"));
        assert_eq!(
            payload.text,
            "이미지\n<notes>.pdf (https://typie.net/files/a.pdf)\nArticle (https://example.com/article)"
        );
        let (restored, source) = Slice::from_payload(Some(&payload.html), &payload.text, &resource);
        assert_eq!(source, PayloadSource::Html);
        assert_eq!(restored, slice);
    }

    #[test]
    fn serialize_empty_slice_with_meta() {
        let slice = Slice::new(vec![], 0, 0);
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(html.contains("data-slice-v2="));
        assert!(html.contains("data-version=\"1\""));
        assert!(html.contains("<div data-root>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn serialize_prepends_charset_meta() {
        let slice = Slice::new(vec![], 0, 0);
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(html.starts_with(r#"<meta charset="utf-8">"#));
    }

    #[test]
    fn serialize_paragraph_with_text() {
        let (s, ..) = state! {
            doc { r: root { paragraph { text("Hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let slice = Slice::extract(&s).unwrap();
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<p>Hello</p>"));
    }

    #[test]
    fn serialize_text_with_bold_and_italic() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "bold italic".into(),
                    }))
                    .with_modifiers(vec![Modifier::Bold, Modifier::Italic]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<strong><em>bold italic</em></strong>"));
    }

    #[test]
    fn serialize_text_with_style_modifiers() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "styled".into(),
                    }))
                    .with_modifiers(vec![
                        Modifier::FontSize { value: 1600 },
                        Modifier::TextColor {
                            value: "#ff0000".into(),
                        },
                    ]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(
            html.contains(r#"<span style="font-size:16pt;color:#ff0000">styled</span>"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_palette_keys_resolve_to_theme_hex() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "colored".into(),
                    }))
                    .with_modifiers(vec![
                        Modifier::TextColor {
                            value: "red".into(),
                        },
                        Modifier::BackgroundColor {
                            value: "yellow".into(),
                        },
                    ]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(
            html.contains(r#"<span style="color:#ef4444;background-color:#fef3c7">colored</span>"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_background_none_resolves_to_transparent() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "plain".into(),
                    }))
                    .with_modifiers(vec![Modifier::BackgroundColor {
                        value: "none".into(),
                    }]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(
            html.contains(r#"background-color:transparent"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_text_with_link() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "click".into(),
                    }))
                    .with_modifiers(vec![Modifier::Link {
                        href: "https://example.com".into(),
                    }]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        assert!(
            slice
                .to_html(&Resource::new_test(), &[])
                .contains(r#"<a href="https://example.com">click</a>"#)
        );
    }

    #[test]
    fn serialize_ruby_preserves_annotation_styles_and_internal_slice() {
        let (state, ..) = state! {
            doc { root { p: paragraph {
                text("前 ")
                text("東京") [
                    bold,
                    link(href: "https://example.com".to_string()),
                    ruby(text: "とう<きょう>&".to_string())
                ]
                text(" 後")
            } } }
            selection: (p, 2) -> (p, 4)
        };
        let slice = Slice::extract(&state).unwrap();
        let resource = Resource::new_test();
        let payload = slice.to_payload(&resource, &[]);
        let document = scraper::Html::parse_fragment(&payload.html);
        let ruby = document
            .select(&scraper::Selector::parse("ruby").unwrap())
            .next()
            .expect("clipboard HTML must expose the ruby annotation");
        let base = ruby
            .select(&scraper::Selector::parse("strong a").unwrap())
            .next()
            .unwrap();
        assert_eq!(base.text().collect::<String>(), "東京");
        assert_eq!(base.value().attr("href"), Some("https://example.com"));
        let annotation = ruby
            .select(&scraper::Selector::parse("rt").unwrap())
            .next()
            .unwrap();
        assert_eq!(annotation.text().collect::<String>(), "とう<きょう>&");
        assert_eq!(annotation.child_elements().count(), 0);
        assert_eq!(payload.text, "東京");
        let (restored, _) = Slice::from_payload(Some(&payload.html), &payload.text, &resource);
        assert_eq!(restored, slice);
    }

    #[test]
    fn ruby_group_survives_base_style_changes_but_stops_at_breaks() {
        let (state, ..) = state! {
            doc { root { p: paragraph {
                text("東") [bold, ruby(text: "とうきょう".to_string())]
                text("京") [link(href: "https://example.com".to_string()), ruby(text: "とうきょう".to_string())]
                hard_break
                text("東") [ruby(text: "とうきょう".to_string())]
                text("京") [italic, ruby(text: "とうきょう".to_string())]
            } } }
            selection: (p, 0) -> (p, 5)
        };
        let slice = Slice::extract(&state).unwrap();
        let resource = Resource::new_test();
        let payload = slice.to_payload(&resource, &[]);
        let html = scraper::Html::parse_fragment(&payload.html);
        let ruby = scraper::Selector::parse("ruby").unwrap();
        let rt = scraper::Selector::parse("rt").unwrap();
        assert_eq!(html.select(&ruby).count(), 2);
        assert_eq!(
            html.select(&rt)
                .map(|node| node.text().collect::<String>())
                .collect::<Vec<_>>(),
            vec!["とうきょう", "とうきょう"]
        );
        assert!(payload.html.contains("<ruby><strong>東</strong><a href=\"https://example.com\">京</a><rt>とうきょう</rt></ruby><br>"));
        assert_eq!(payload.text, "東京\n東京");
        assert_eq!(
            Slice::from_payload(Some(&payload.html), &payload.text, &resource).0,
            slice
        );
    }

    #[test]
    fn serialize_bullet_list() {
        let (s, ..) = state! {
            doc { r: root {
                bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let slice = Slice::extract(&s).unwrap();
        let html = slice.to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("<p>a</p>"));
    }

    #[test]
    fn serialize_multi_paragraph_list_item_uses_one_li() {
        let (s, ..) = state! {
            doc { r: root {
                bullet_list {
                    list_item {
                        paragraph { text("a") }
                        paragraph { text("b") }
                    }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        let html = Slice::extract(&s)
            .unwrap()
            .to_html(&Resource::new_test(), &[]);

        assert_eq!(html.matches("<li>").count(), 1);
        assert!(html.contains("<p>a</p>"));
        assert!(html.contains("<p>b</p>"));
        assert!(html.find("<p>a</p>").unwrap() < html.find("<p>b</p>").unwrap());
    }

    #[test]
    fn serialize_table() {
        let (s, ..) = state! {
            doc { r: root {
                table {
                    table_row {
                        table_cell { paragraph { text("a") } }
                        table_cell { paragraph { text("b") } }
                    }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let html = Slice::extract(&s)
            .unwrap()
            .to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<table"));
        assert!(html.contains("<tr>"));
        assert!(html.contains("<td"));
        assert!(
            html.contains(r#"data-border-style="solid""#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_image() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        b.image(&[root]);
        let s = b.finish(Some(Selection::new(
            Position::new(root, 0),
            Position::new(root, 1),
        )));
        let html = Slice::extract(&s)
            .unwrap()
            .to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<img data-id"));
    }

    #[test]
    fn serialize_horizontal_rule() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let _p1 = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        b.horizontal_rule(&[root]);
        let _p2 = b.block(NodeType::Paragraph, &[root]);
        b.text("b");
        let s = b.finish(Some(Selection::new(
            Position::new(root, 1),
            Position::new(root, 2),
        )));
        let html = Slice::extract(&s)
            .unwrap()
            .to_html(&Resource::new_test(), &[]);
        assert!(html.contains("<hr>"));
    }
}
