use super::markup::{
    HtmlTarget, NodeMarkup, dom_projection_style, markup_for_node, write_close_tag,
    write_dom_projection_text, write_open_tag,
};
use editor_model::{PlainDoc, PlainNode, PlainNodeEntry, Schema};
use editor_resource::Resource;

pub fn serialize(doc: &PlainDoc, resource: &Resource) -> String {
    let mut out = String::new();
    serialize_root(&doc.root, resource, &mut out);
    out
}

enum SerializeTask<'a> {
    Node {
        entry: &'a PlainNodeEntry,
        path: String,
    },
    Close(&'static str),
}

fn serialize_root(root: &PlainNodeEntry, resource: &Resource, out: &mut String) {
    let mut tasks = vec![SerializeTask::Node {
        entry: root,
        path: "r".to_string(),
    }];

    while let Some(task) = tasks.pop() {
        let (entry, path) = match task {
            SerializeTask::Node { entry, path } => (entry, path),
            SerializeTask::Close(tag) => {
                write_close_tag(out, tag);
                continue;
            }
        };

        match markup_for_node(&entry.node, HtmlTarget::DomProjection) {
            NodeMarkup::Text => {
                let PlainNode::Text(text) = &entry.node else {
                    unreachable!()
                };
                write_dom_projection_text(
                    &text.text,
                    entry.modifiers.values(),
                    resource,
                    &path,
                    out,
                );
            }
            NodeMarkup::Tab => unreachable!("DOM projection tabs use element markup"),
            NodeMarkup::Children => push_children(&mut tasks, entry, &path),
            NodeMarkup::Skip => {}
            NodeMarkup::Element { tag, attrs, void } => {
                let style = dom_projection_style(entry.modifiers.values(), resource);
                let inline = Schema::node_spec(entry.node.as_type()).inline;
                let inline_container = !entry.children.is_empty()
                    && entry
                        .children
                        .iter()
                        .all(|child| Schema::node_spec(child.node.as_type()).inline);
                let mut extra_attrs = vec![("data-typie-node", Some(path.as_str()))];
                if inline_container {
                    extra_attrs.push(("data-typie-text-container", None));
                }
                if inline {
                    extra_attrs.push(("data-typie-boundary", None));
                }
                if matches!(entry.node, PlainNode::Fold(_)) {
                    extra_attrs.push(("open", None));
                }
                if let Some(style) = style.as_deref() {
                    extra_attrs.push(("style", Some(style)));
                }

                write_open_tag(out, tag, &attrs, &extra_attrs);
                if !void {
                    tasks.push(SerializeTask::Close(tag));
                    push_children(&mut tasks, entry, &path);
                }
            }
        }
    }
}

fn push_children<'a>(tasks: &mut Vec<SerializeTask<'a>>, entry: &'a PlainNodeEntry, path: &str) {
    for (index, child) in entry.children.iter().enumerate().rev() {
        let child_path = if path == "r" {
            index.to_string()
        } else {
            format!("{path}.{index}")
        };
        tasks.push(SerializeTask::Node {
            entry: child,
            path: child_path,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    #[test]
    fn marks_schema_inline_content_for_dom_projection() {
        let (state, ..) = state! {
            doc { root { p: paragraph { text("Before") page_break } } }
            selection: (p, 0)
        };

        let html = serialize(&state.to_plain(), &Resource::new_test());

        assert!(
            html.starts_with(r#"<article data-typie-node="r""#),
            "actual: {html}"
        );
        assert!(html.contains(r#"data-typie-text-container"#));
        assert!(html.contains(r#"data-typie-text="0.0""#));
        assert!(html.contains(r#"data-typie-node="0.1" data-typie-boundary"#));
    }

    #[test]
    fn uses_canonical_html_modifiers_and_theme_colors() {
        let (state, ..) = state! {
            doc { root { p: paragraph [line_height(160), paragraph_indent(100), alignment(editor_model::Alignment::Center)] {
                text("Tokyo") [bold, text_color("red".to_string()), ruby(text: "とうきょう".to_string())]
            } } }
            selection: (p, 0)
        };

        let html = serialize(&state.to_plain(), &Resource::new_test());

        assert!(html.contains("<strong>"));
        assert!(html.contains("color:#ef4444"));
        assert!(html.contains("line-height:1.6"));
        assert!(html.contains("text-indent:16px"));
        assert!(html.contains("text-align:center"));
        assert!(html.contains("<ruby>"));
        assert!(html.contains(r#"<rt translate="no">とうきょう</rt>"#));
    }

    #[test]
    fn does_not_treat_modifier_values_as_css_declarations() {
        let (state, ..) = state! {
            doc { root { p: paragraph {
                text("Styled") [
                    font_family("Injected; background-image:url(https://example.com/font)".to_string()),
                    text_color("red;background-image:url(https://example.com/color)".to_string())
                ]
            } } }
            selection: (p, 0)
        };

        let html = serialize(&state.to_plain(), &Resource::new_test());
        let document = scraper::Html::parse_fragment(&html);
        let selector = scraper::Selector::parse("[data-typie-text]").unwrap();
        let style = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("style"))
            .expect("document text must have a style attribute");
        let properties = crate::html::parse::stylesheet::parse_inline_style(style)
            .into_iter()
            .map(|declaration| declaration.property)
            .collect::<Vec<_>>();

        assert_eq!(properties, ["font-family"]);
    }
}
