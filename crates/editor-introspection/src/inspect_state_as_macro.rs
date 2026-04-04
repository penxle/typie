use editor_model::*;
use editor_state::{Affinity, PendingModifier, State};
use std::fmt::Write;

use crate::labeler::Labeler;

pub fn inspect_state_as_macro(state: &State) -> String {
    let labeler = Labeler::new(&state.doc, &state.selection);
    let mut output = String::new();

    output.push_str("state! {\n");
    write_indent(&mut output, 1);
    output.push_str("doc {\n");

    let root = state.doc.root();
    let children: Vec<_> = root.children().collect();

    write_indent(&mut output, 2);
    output.push_str("root");
    write_modifiers_macro(&non_default_root_modifiers(root.modifiers()), &mut output);
    if children.is_empty() {
        output.push_str(" {}\n");
    } else {
        output.push_str(" {\n");
        for child in &children {
            write_macro_node(child, 3, &labeler, &mut output);
        }
        write_indent(&mut output, 2);
        output.push_str("}\n");
    }

    write_indent(&mut output, 1);
    output.push_str("}\n");
    write_selection_macro(&state.selection, &labeler, &mut output);
    write_pending_modifiers(&state.pending_modifiers, &mut output);
    output.push_str("}\n");
    output
}

fn write_indent(output: &mut String, level: usize) {
    const INDENT: &str = "    ";
    for _ in 0..level {
        output.push_str(INDENT);
    }
}

fn write_macro_node(
    node_ref: &NodeRef,
    indent_level: usize,
    labeler: &Labeler,
    output: &mut String,
) {
    write_indent(output, indent_level);

    if let Some(l) = labeler.label(node_ref.id()) {
        write!(output, "{l}: ").unwrap();
    }

    let type_name: &str = node_ref.as_type().into();
    write!(output, "{type_name}").unwrap();

    if let Node::Text(t) = node_ref.node() {
        write!(output, "(\"{}\")", escape_str(&t.text)).unwrap();
    }

    write_node_attrs_macro(node_ref.node(), output);
    write_modifiers_macro(node_ref.modifiers(), output);

    let children: Vec<_> = node_ref.children().collect();
    if matches!(node_ref.node(), Node::Text(_)) {
        output.push('\n');
    } else if children.is_empty() {
        output.push_str(" {}\n");
    } else {
        output.push_str(" {\n");
        for child in &children {
            write_macro_node(child, indent_level + 1, labeler, output);
        }
        write_indent(output, indent_level);
        output.push_str("}\n");
    }
}

fn write_selection_macro(
    selection: &editor_state::Selection,
    labeler: &Labeler,
    output: &mut String,
) {
    let show_affinity = selection.anchor.affinity != Affinity::Downstream
        || selection.head.affinity != Affinity::Downstream;

    write_indent(output, 1);
    output.push_str("selection: (");
    write_position_macro(&selection.anchor, show_affinity, labeler, output);
    output.push(')');

    if !selection.is_collapsed() {
        output.push_str(" -> (");
        write_position_macro(&selection.head, show_affinity, labeler, output);
        output.push(')');
    }
    output.push('\n');
}

fn write_position_macro(
    pos: &editor_state::Position,
    show_affinity: bool,
    labeler: &Labeler,
    output: &mut String,
) {
    match labeler.label(pos.node_id) {
        Some(l) => write!(output, "{l}").unwrap(),
        None => write!(output, "{}", pos.node_id).unwrap(),
    }
    write!(output, ", {}", pos.offset).unwrap();
    if show_affinity {
        let aff = match pos.affinity {
            Affinity::Downstream => ">",
            Affinity::Upstream => "<",
        };
        write!(output, ", {aff}").unwrap();
    }
}

fn write_pending_modifiers(pending: &editor_state::PendingModifiers, output: &mut String) {
    if pending.is_empty() {
        return;
    }
    write_indent(output, 1);
    output.push_str("pending_modifiers: [");
    for (i, pm) in pending.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        match pm {
            PendingModifier::Set(m) => write_modifier_macro(m, output),
            PendingModifier::Unset(t) => {
                let name: &str = (*t).into();
                write!(output, "!{name}").unwrap();
            }
        }
    }
    output.push_str("]\n");
}

fn write_node_attrs_macro(node: &Node, output: &mut String) {
    let mut attrs = Vec::new();
    match node {
        Node::Paragraph(p) if p.align != TextAlign::default() => {
            attrs.push(format!("align: TextAlign::{:?}", p.align));
        }
        Node::Blockquote(bq) if bq.variant != BlockquoteVariant::default() => {
            attrs.push(format!("variant: BlockquoteVariant::{:?}", bq.variant));
        }
        Node::Callout(c) if c.variant != CalloutVariant::default() => {
            attrs.push(format!("variant: CalloutVariant::{:?}", c.variant));
        }
        Node::HorizontalRule(hr) if hr.variant != HorizontalRuleVariant::default() => {
            attrs.push(format!("variant: HorizontalRuleVariant::{:?}", hr.variant));
        }
        Node::Table(t) => {
            if t.border_style != TableBorderStyle::default() {
                attrs.push(format!(
                    "border_style: TableBorderStyle::{:?}",
                    t.border_style
                ));
            }
            if t.align != TableAlign::default() {
                attrs.push(format!("align: TableAlign::{:?}", t.align));
            }
            if (t.proportion - 1.0).abs() > f32::EPSILON {
                attrs.push(format!("proportion: {}", format_f32(t.proportion)));
            }
        }
        Node::TableCell(tc) => {
            if let Some(w) = tc.col_width {
                attrs.push(format!("col_width: Some({})", format_f32(w)));
            }
        }
        Node::Image(img) => {
            if let Some(id) = &img.id {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
            if (img.proportion - 1.0).abs() > f32::EPSILON {
                attrs.push(format!("proportion: {}", format_f32(img.proportion)));
            }
        }
        Node::File(f) => {
            if let Some(id) = &f.id {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
        }
        Node::Embed(e) => {
            if let Some(id) = &e.id {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
        }
        Node::Archived(a) => {
            if let Some(id) = &a.id {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
        }
        _ => {}
    }
    if !attrs.is_empty() {
        write!(output, "({})", attrs.join(", ")).unwrap();
    }
}

fn write_modifier_macro(m: &Modifier, output: &mut String) {
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
            write!(output, "{name}(\"{value}\".to_string())").unwrap();
        }
        Modifier::Link { href } => {
            write!(output, "{name}(href: \"{href}\".to_string())").unwrap();
        }
        Modifier::Ruby { text } => {
            write!(output, "{name}(text: \"{text}\".to_string())").unwrap();
        }
    }
}

fn write_modifiers_macro(modifiers: &[Modifier], output: &mut String) {
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

fn escape_str(s: &str) -> String {
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

fn non_default_root_modifiers(modifiers: &[Modifier]) -> Vec<Modifier> {
    let defaults = editor_model::default_modifiers();
    modifiers
        .iter()
        .filter(|m| !defaults.contains(m))
        .cloned()
        .collect()
}

fn format_f32(v: f32) -> String {
    if (v - v.round()).abs() < f32::EPSILON {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::inspect_state_as_macro;

    #[test]
    fn simple_state() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let output = inspect_state_as_macro(&state);
        let expected = "\
state! {
    doc {
        root {
            paragraph {
                t1: text(\"Hello\")
            }
        }
    }
    selection: (t1, 3)
}
";
        assert_eq!(output, expected);
    }

    #[test]
    fn range_selection() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 3)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (t1, 0) -> (t2, 3)"));
    }

    #[test]
    fn empty_container() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("p1: paragraph {}"));
    }

    #[test]
    fn text_escaping() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("He said \"hi\"\nnewline") } } }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains(r#"text("He said \"hi\"\nnewline")"#));
    }

    #[test]
    fn modifiers_output() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold, italic] } } }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("t1: text(\"Hello\") [bold, italic]"));
    }

    #[test]
    fn non_default_paragraph_align() {
        let (state, ..) = state! {
            doc { root { paragraph(align: TextAlign::Center) { t1: text("Hi") } } }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("paragraph(align: TextAlign::Center)"));
    }

    #[test]
    fn default_blockquote_variant_omitted() {
        let (state, ..) = state! {
            doc { root { blockquote { p1: paragraph {} } } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("blockquote {"));
        assert!(!output.contains("BlockquoteVariant"));
    }

    #[test]
    fn non_default_blockquote_variant_shown() {
        let (state, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph {}
                    }
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("blockquote(variant: BlockquoteVariant::MessageSent)"));
    }

    #[test]
    fn affinity_omitted_when_both_downstream() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 3)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (t1, 0) -> (t2, 3)"));
        assert!(!output.contains(", >"));
        assert!(!output.contains(", <"));
    }

    #[test]
    fn affinity_shown_when_non_default() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0, <) -> (t2, 3)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (t1, 0, <) -> (t2, 3, >)"));
    }

    #[test]
    fn root_default_modifiers_omitted() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        let root_line = output
            .lines()
            .find(|l| l.trim().starts_with("root"))
            .unwrap();
        assert!(!root_line.contains("["));
    }

    #[test]
    fn root_non_default_modifiers_shown() {
        let (state, ..) = state! {
            doc { root [font_size(1600)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("root [font_size(1600)]"));
    }

    #[test]
    fn pending_modifiers() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold, !italic]
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("pending_modifiers: [bold, !italic]"));
    }

    #[test]
    fn struct_variant_modifier() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Click") [link(href: "https://example.com".to_string())]
                    }
                }
            }
            selection: (t1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("text(\"Click\") [link(href: \"https://example.com\".to_string())]")
        );
    }
}
