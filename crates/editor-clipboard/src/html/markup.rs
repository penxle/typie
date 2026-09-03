use editor_model::{Modifier, PlainNode};
use editor_resource::Resource;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HtmlTarget {
    Clipboard,
    DomProjection,
}

pub(super) enum NodeMarkup {
    Text,
    Tab,
    Children,
    Skip,
    Element {
        tag: &'static str,
        attrs: String,
        void: bool,
    },
}

fn element(tag: &'static str) -> NodeMarkup {
    NodeMarkup::Element {
        tag,
        attrs: String::new(),
        void: false,
    }
}

fn void_element(tag: &'static str) -> NodeMarkup {
    NodeMarkup::Element {
        tag,
        attrs: String::new(),
        void: true,
    }
}

pub(super) fn markup_for_node(node: &PlainNode, target: HtmlTarget) -> NodeMarkup {
    match node {
        PlainNode::Text(_) => NodeMarkup::Text,
        PlainNode::HardBreak(_) => void_element("br"),
        PlainNode::Tab(_) if target == HtmlTarget::Clipboard => NodeMarkup::Tab,
        PlainNode::Tab(_) => element("span"),
        PlainNode::Paragraph(_) => element("p"),
        PlainNode::BulletList(_) => element("ul"),
        PlainNode::OrderedList(_) => element("ol"),
        PlainNode::ListItem(_) => element("li"),
        PlainNode::Blockquote(blockquote) => NodeMarkup::Element {
            tag: "blockquote",
            attrs: format!(r#"data-variant="{}""#, variant_str(&blockquote.variant)),
            void: false,
        },
        PlainNode::Callout(callout) => NodeMarkup::Element {
            tag: "aside",
            attrs: format!(
                r#"data-callout data-variant="{}""#,
                variant_str(&callout.variant)
            ),
            void: false,
        },
        PlainNode::Fold(_) => element("details"),
        PlainNode::FoldTitle(_) => element("summary"),
        PlainNode::FoldContent(_) if target == HtmlTarget::Clipboard => NodeMarkup::Children,
        PlainNode::FoldContent(_) => element("div"),
        PlainNode::Table(table) => NodeMarkup::Element {
            tag: "table",
            attrs: format!(
                r#"data-border-style="{}" data-proportion="{}""#,
                variant_str(&table.border_style),
                table.proportion,
            ),
            void: false,
        },
        PlainNode::TableRow(_) => element("tr"),
        PlainNode::TableCell(cell) => NodeMarkup::Element {
            tag: "td",
            attrs: cell
                .col_width
                .map(|width| format!(r#"data-col-width="{width}""#))
                .unwrap_or_default(),
            void: false,
        },
        PlainNode::Image(image) if target == HtmlTarget::Clipboard => NodeMarkup::Element {
            tag: "img",
            attrs: format!(
                r#"data-id="{}" data-proportion="{}""#,
                html_escape(image.id.as_deref().unwrap_or("")),
                image.proportion,
            ),
            void: true,
        },
        PlainNode::Image(image) => NodeMarkup::Element {
            tag: "figure",
            attrs: format!(
                r#"data-id="{}" data-proportion="{}""#,
                html_escape(image.id.as_deref().unwrap_or("")),
                image.proportion,
            ),
            void: false,
        },
        PlainNode::Embed(embed) if target == HtmlTarget::Clipboard => NodeMarkup::Element {
            tag: "a",
            attrs: format!(
                r#"data-embed data-id="{}""#,
                html_escape(embed.id.as_deref().unwrap_or("")),
            ),
            void: false,
        },
        PlainNode::Embed(embed) => NodeMarkup::Element {
            tag: "div",
            attrs: format!(
                r#"data-embed data-id="{}""#,
                html_escape(embed.id.as_deref().unwrap_or("")),
            ),
            void: false,
        },
        PlainNode::File(file) if target == HtmlTarget::Clipboard => NodeMarkup::Element {
            tag: "a",
            attrs: format!(
                r#"data-file data-id="{}""#,
                html_escape(file.id.as_deref().unwrap_or("")),
            ),
            void: false,
        },
        PlainNode::File(file) => NodeMarkup::Element {
            tag: "div",
            attrs: format!(
                r#"data-file data-id="{}""#,
                html_escape(file.id.as_deref().unwrap_or("")),
            ),
            void: false,
        },
        PlainNode::Archived(_) if target == HtmlTarget::Clipboard => NodeMarkup::Skip,
        PlainNode::Archived(_) => element("div"),
        PlainNode::PageBreak(_) if target == HtmlTarget::Clipboard => NodeMarkup::Element {
            tag: "div",
            attrs: r#"style="page-break-after:always""#.to_string(),
            void: false,
        },
        PlainNode::PageBreak(_) => NodeMarkup::Element {
            tag: "span",
            attrs: r#"style="break-after:page""#.to_string(),
            void: false,
        },
        PlainNode::HorizontalRule(_) => void_element("hr"),
        PlainNode::Root(_) if target == HtmlTarget::Clipboard => NodeMarkup::Children,
        PlainNode::Root(_) => element("article"),
        PlainNode::Unknown if target == HtmlTarget::Clipboard => NodeMarkup::Skip,
        PlainNode::Unknown => element("div"),
    }
}

pub(super) fn write_open_tag(
    out: &mut String,
    tag: &str,
    attrs: &str,
    extra_attrs: &[(&str, Option<&str>)],
) {
    out.push('<');
    out.push_str(tag);
    if !attrs.is_empty() {
        out.push(' ');
        out.push_str(attrs);
    }
    for (name, value) in extra_attrs {
        out.push(' ');
        out.push_str(name);
        if let Some(value) = value {
            out.push_str(r#"=""#);
            out.push_str(&html_escape(value));
            out.push('"');
        }
    }
    out.push('>');
}

pub(super) fn write_close_tag(out: &mut String, tag: &str) {
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

pub(super) fn write_clipboard_text(
    text: &str,
    modifiers: &[Modifier],
    resource: &Resource,
    out: &mut String,
) {
    let presentation = modifier_presentation(modifiers, resource, HtmlTarget::Clipboard);
    for structural in &presentation.structural {
        structural.write_open(out);
    }
    let style = joined_style(&presentation);
    if let Some(style) = style.as_deref() {
        write_open_tag(out, "span", "", &[("style", Some(style))]);
    }
    out.push_str(&html_escape(text));
    if style.is_some() {
        write_close_tag(out, "span");
    }
    for structural in presentation.structural.iter().rev() {
        structural.write_close(out);
    }
}

pub(super) fn write_dom_projection_text<'m>(
    text: &str,
    modifiers: impl IntoIterator<Item = &'m Modifier>,
    resource: &Resource,
    path: &str,
    out: &mut String,
) {
    let presentation = modifier_presentation(modifiers, resource, HtmlTarget::DomProjection);
    if presentation.ruby.is_some() {
        out.push_str("<ruby>");
    }
    for structural in &presentation.structural {
        structural.write_open(out);
    }

    let style = joined_style(&presentation);
    let mut attrs = vec![("data-typie-text", Some(path))];
    if let Some(style) = style.as_deref() {
        attrs.push(("style", Some(style)));
    }
    write_open_tag(out, "span", "", &attrs);
    out.push_str(&html_escape(text));
    write_close_tag(out, "span");

    for structural in presentation.structural.iter().rev() {
        structural.write_close(out);
    }
    if let Some(ruby) = presentation.ruby {
        out.push_str(r#"<rt translate="no">"#);
        out.push_str(&html_escape(ruby));
        out.push_str("</rt></ruby>");
    }
}

pub(super) fn dom_projection_style<'m>(
    modifiers: impl IntoIterator<Item = &'m Modifier>,
    resource: &Resource,
) -> Option<String> {
    joined_style(&modifier_presentation(
        modifiers,
        resource,
        HtmlTarget::DomProjection,
    ))
}

enum StructuralMarkup<'a> {
    Strong,
    Emphasis,
    Underline,
    Strikethrough,
    Link(&'a str),
}

impl StructuralMarkup<'_> {
    fn order(&self) -> u8 {
        match self {
            Self::Strong => 0,
            Self::Emphasis => 1,
            Self::Underline => 2,
            Self::Strikethrough => 3,
            Self::Link(_) => 4,
        }
    }

    fn write_open(&self, out: &mut String) {
        match self {
            Self::Strong => out.push_str("<strong>"),
            Self::Emphasis => out.push_str("<em>"),
            Self::Underline => out.push_str("<u>"),
            Self::Strikethrough => out.push_str("<s>"),
            Self::Link(href) => {
                out.push_str("<a href=\"");
                out.push_str(&html_escape(href));
                out.push_str(r#"">"#);
            }
        }
    }

    fn write_close(&self, out: &mut String) {
        match self {
            Self::Strong => out.push_str("</strong>"),
            Self::Emphasis => out.push_str("</em>"),
            Self::Underline => out.push_str("</u>"),
            Self::Strikethrough => out.push_str("</s>"),
            Self::Link(_) => out.push_str("</a>"),
        }
    }
}

struct ModifierPresentation<'a> {
    structural: Vec<StructuralMarkup<'a>>,
    style: Vec<String>,
    ruby: Option<&'a str>,
}

fn joined_style(presentation: &ModifierPresentation<'_>) -> Option<String> {
    (!presentation.style.is_empty()).then(|| presentation.style.join(";"))
}

fn css_color(value: &str, token_prefix: &str, resource: &Resource) -> Option<String> {
    if value == "none" {
        return Some("transparent".to_string());
    }
    match resource
        .theme()
        .try_color(&format!("{token_prefix}.{value}"))
    {
        Some(c) => Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
        None => {
            let [r, g, b, a] = csscolorparser::parse(value.trim()).ok()?.to_rgba8();
            Some(if a == u8::MAX {
                format!("#{r:02x}{g:02x}{b:02x}")
            } else {
                format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
            })
        }
    }
}

fn css_font_family(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let mut serialized = String::new();
    cssparser::serialize_string(value, &mut serialized).expect("writing to a String cannot fail");
    Some(serialized)
}

fn modifier_presentation<'m>(
    modifiers: impl IntoIterator<Item = &'m Modifier>,
    resource: &Resource,
    target: HtmlTarget,
) -> ModifierPresentation<'m> {
    let mut structural = Vec::new();
    let mut style = Vec::new();
    let mut ruby = None;
    for modifier in modifiers {
        match modifier {
            Modifier::Bold => structural.push(StructuralMarkup::Strong),
            Modifier::Italic => structural.push(StructuralMarkup::Emphasis),
            Modifier::Underline => structural.push(StructuralMarkup::Underline),
            Modifier::Strikethrough => structural.push(StructuralMarkup::Strikethrough),
            Modifier::FontSize { value } => {
                style.push(format!("font-size:{}pt", *value as f32 / 100.0))
            }
            Modifier::FontFamily { value } => {
                if let Some(value) = css_font_family(value) {
                    style.push(format!("font-family:{value}"));
                }
            }
            Modifier::FontWeight { value } => style.push(format!("font-weight:{value}")),
            Modifier::TextColor { value } => {
                if let Some(value) = css_color(value, "text", resource) {
                    style.push(format!("color:{value}"));
                }
            }
            Modifier::BackgroundColor { value } => {
                if let Some(value) = css_color(value, "bg", resource) {
                    style.push(format!("background-color:{value}"));
                }
            }
            Modifier::LetterSpacing { value } => {
                style.push(format!("letter-spacing:{}em", *value as f32 / 100.0))
            }
            Modifier::Link { href } => structural.push(StructuralMarkup::Link(href)),
            Modifier::Ruby { text } => {
                if target == HtmlTarget::DomProjection {
                    ruby = Some(text.as_str());
                }
            }
            Modifier::LineHeight { value } => {
                if target == HtmlTarget::DomProjection {
                    style.push(format!("line-height:{}", *value as f32 / 100.0));
                }
            }
            Modifier::BlockGap { .. } => {}
            Modifier::ParagraphIndent { value } => {
                if target == HtmlTarget::DomProjection {
                    style.push(format!("text-indent:{}px", *value as f32 / 100.0 * 16.0));
                }
            }
            Modifier::Alignment { value } => {
                if target == HtmlTarget::DomProjection {
                    style.push(format!("text-align:{}", variant_str(value)));
                }
            }
        }
    }
    structural.sort_by_key(StructuralMarkup::order);
    ModifierPresentation {
        structural,
        style,
        ruby,
    }
}

// 변형 enum 들은 #[serde(rename_all = "snake_case")] 의 plain string 직렬화를 가정
fn variant_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(String::from))
        .unwrap_or_default()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
