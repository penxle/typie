use std::fmt::Write;

use editor_model::*;

pub(crate) fn write_indent(output: &mut String, level: usize) {
    const INDENT: &str = "    ";
    for _ in 0..level {
        output.push_str(INDENT);
    }
}

pub(crate) fn write_node_attrs_macro(node: &PlainNode, output: &mut String) {
    let mut attrs = Vec::new();
    match node {
        PlainNode::Root(root) => {
            if root.layout_mode != LayoutMode::default() {
                attrs.push(format!(
                    "layout_mode: {}",
                    layout_mode_expr(root.layout_mode)
                ));
            }
        }
        PlainNode::Blockquote(bq) => {
            if bq.variant != BlockquoteVariant::default() {
                attrs.push(format!("variant: BlockquoteVariant::{:?}", bq.variant));
            }
        }
        PlainNode::Callout(c) => {
            if c.variant != CalloutVariant::default() {
                attrs.push(format!("variant: CalloutVariant::{:?}", c.variant));
            }
        }
        PlainNode::HorizontalRule(hr) => {
            if hr.variant != HorizontalRuleVariant::default() {
                attrs.push(format!("variant: HorizontalRuleVariant::{:?}", hr.variant));
            }
        }
        PlainNode::Table(t) => {
            if t.border_style != TableBorderStyle::default() {
                attrs.push(format!(
                    "border_style: TableBorderStyle::{:?}",
                    t.border_style
                ));
            }
            if t.proportion != 100 {
                attrs.push(format!("proportion: {}", t.proportion));
            }
        }
        PlainNode::TableCell(tc) => {
            if let Some(w) = tc.col_width {
                attrs.push(format!("col_width: Some({w})"));
            }
            if let Some(background_color) = &tc.background_color {
                attrs.push(format!(
                    "background_color: Some({})",
                    owned_string_expr(background_color)
                ));
            }
        }
        PlainNode::Image(img) => {
            if let Some(id) = &img.id {
                attrs.push(format!("id: Some({})", owned_string_expr(id)));
            }
            if img.proportion != 100 {
                attrs.push(format!("proportion: {}", img.proportion));
            }
        }
        PlainNode::File(f) => {
            if let Some(id) = &f.id {
                attrs.push(format!("id: Some({})", owned_string_expr(id)));
            }
        }
        PlainNode::Embed(e) => {
            if let Some(id) = &e.id {
                attrs.push(format!("id: Some({})", owned_string_expr(id)));
            }
        }
        PlainNode::Archived(a) => {
            if let Some(id) = &a.id {
                attrs.push(format!("id: Some({})", owned_string_expr(id)));
            }
        }
        PlainNode::Paragraph(_)
        | PlainNode::Text(_)
        | PlainNode::BulletList(_)
        | PlainNode::OrderedList(_)
        | PlainNode::ListItem(_)
        | PlainNode::Fold(_)
        | PlainNode::FoldTitle(_)
        | PlainNode::FoldContent(_)
        | PlainNode::TableRow(_)
        | PlainNode::HardBreak(_)
        | PlainNode::PageBreak(_)
        | PlainNode::Tab(_)
        | PlainNode::Unknown => {}
    }
    if !attrs.is_empty() {
        write!(output, "({})", attrs.join(", ")).unwrap();
    }
}

pub(crate) fn write_modifier_macro(m: &Modifier, output: &mut String) {
    let name: &str = m.as_type().into();
    match m {
        Modifier::Bold | Modifier::Italic | Modifier::Underline | Modifier::Strikethrough => {
            output.push_str(name);
        }
        Modifier::FontSize { value }
        | Modifier::LineHeight { value }
        | Modifier::BlockGap { value }
        | Modifier::ParagraphIndent { value } => write!(output, "{name}({value})").unwrap(),
        Modifier::FontWeight { value } => write!(output, "{name}({value})").unwrap(),
        Modifier::LetterSpacing { value } => write!(output, "{name}({value})").unwrap(),
        Modifier::FontFamily { value }
        | Modifier::TextColor { value }
        | Modifier::BackgroundColor { value } => {
            write!(output, "{name}({})", owned_string_expr(value)).unwrap();
        }
        Modifier::Link { href } => {
            write!(output, "{name}(href: {})", owned_string_expr(href)).unwrap();
        }
        Modifier::Ruby { text } => {
            write!(output, "{name}(text: {})", owned_string_expr(text)).unwrap();
        }
        Modifier::Alignment { value } => {
            write!(output, "{name}(Alignment::{value:?})").unwrap();
        }
    }
}

pub(crate) fn write_modifiers_macro(modifiers: &[Modifier], output: &mut String) {
    if modifiers.is_empty() {
        return;
    }
    output.push_str(" [");
    for (i, m) in modifiers.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_modifier_macro(m, output);
    }
    output.push(']');
}

pub(crate) fn write_carry_macro(carry: &[Modifier], output: &mut String) {
    if carry.is_empty() {
        return;
    }
    output.push_str(" carry([");
    for (i, modifier) in carry.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_modifier_macro(modifier, output);
    }
    output.push_str("])");
}

pub(crate) fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => write!(out, "\\u{{{:x}}}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out
}

fn owned_string_expr(value: &str) -> String {
    format!("\"{}\".to_string()", escape_str(value))
}

fn layout_mode_expr(layout_mode: LayoutMode) -> String {
    match layout_mode {
        LayoutMode::Paginated {
            page_width,
            page_height,
            page_margin_top,
            page_margin_bottom,
            page_margin_left,
            page_margin_right,
        } => format!(
            "LayoutMode::Paginated {{ page_width: {page_width}, page_height: {page_height}, \
             page_margin_top: {page_margin_top}, page_margin_bottom: {page_margin_bottom}, \
             page_margin_left: {page_margin_left}, page_margin_right: {page_margin_right} }}"
        ),
        LayoutMode::Continuous { max_width } => {
            format!("LayoutMode::Continuous {{ max_width: {max_width} }}")
        }
    }
}
