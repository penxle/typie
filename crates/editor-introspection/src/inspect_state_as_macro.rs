use editor_model::*;
use editor_state::{Affinity, PendingModifier, State};
use std::fmt::Write;

use crate::labeler::Labeler;

enum Disp<'a> {
    Block(NodeView<'a>),
    Text {
        text: String,
        modifiers: Vec<Modifier>,
        style: Option<String>,
    },
    Atom(LeafView<'a>),
}

fn display_children<'a>(node: &NodeView<'a>, pd: &ProjectedDoc) -> Vec<Disp<'a>> {
    let mut out: Vec<Disp<'a>> = Vec::new();
    let mut run: Option<(String, Vec<Modifier>, Option<String>)> = None;
    for child in node.children() {
        match child {
            ChildView::Block(b) => {
                if let Some((text, modifiers, style)) = run.take() {
                    out.push(Disp::Text {
                        text,
                        modifiers,
                        style,
                    });
                }
                out.push(Disp::Block(b));
            }
            ChildView::Leaf(l) => match l.as_char() {
                Some(c) => {
                    let modifiers = explicit_leaf_mods(pd, l.dot());
                    let style = pd.node_styles.get(&l.dot()).and_then(|o| o.clone());
                    let extend = matches!(&run, Some((_, m, s)) if *m == modifiers && *s == style);
                    if extend {
                        if let Some((text, _, _)) = run.as_mut() {
                            text.push(c);
                        }
                    } else {
                        if let Some((text, modifiers, style)) = run.take() {
                            out.push(Disp::Text {
                                text,
                                modifiers,
                                style,
                            });
                        }
                        run = Some((c.to_string(), modifiers, style));
                    }
                }
                None => {
                    if let Some((text, modifiers, style)) = run.take() {
                        out.push(Disp::Text {
                            text,
                            modifiers,
                            style,
                        });
                    }
                    out.push(Disp::Atom(l));
                }
            },
        }
    }
    if let Some((text, modifiers, style)) = run.take() {
        out.push(Disp::Text {
            text,
            modifiers,
            style,
        });
    }
    out
}

fn atom_node(leaf: &LeafView, pd: &ProjectedDoc) -> Node {
    pd.node_attrs
        .get(&leaf.dot())
        .cloned()
        .unwrap_or_else(|| leaf.as_atom().expect("atom leaf").clone().into_node())
}

/// Modifiers explicitly set on a block (`SetModifier`), excluding inherited and
/// schema-default modifiers — required for the emitted macro to round-trip.
fn explicit_block_mods(pd: &ProjectedDoc, dot: editor_crdt::Dot) -> Vec<Modifier> {
    let mut mods: Vec<Modifier> = pd
        .block_modifiers
        .get(&dot)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    mods.sort_by_key(|m| m.as_type());
    mods
}

/// Modifiers explicitly applied to a leaf via spans (excluding style-derived and
/// inherited modifiers).
fn explicit_leaf_mods(pd: &ProjectedDoc, dot: editor_crdt::Dot) -> Vec<Modifier> {
    let mut mods: Vec<Modifier> = pd
        .own_modifiers
        .get(&dot)
        .map(|m| {
            m.values()
                .filter(|o| !o.from_style)
                .map(|o| o.value.clone())
                .collect()
        })
        .unwrap_or_default();
    mods.sort_by_key(|m| m.as_type());
    mods
}

pub fn inspect_state_as_macro(state: &State) -> String {
    let view = state.view();
    let pd = state.projected.projected();
    let labeler = Labeler::new(&view, state.selection.as_ref());
    let mut output = String::new();

    output.push_str("state! {\n");
    write_indent(&mut output, 1);
    output.push_str("doc {\n");

    write_styles_macro(pd, &mut output);

    let root = view.root().unwrap();
    let children = display_children(&root, pd);

    write_indent(&mut output, 2);
    if let Some(l) = labeler.label(root.id()) {
        write!(output, "{l}: ").unwrap();
    }
    output.push_str("root");
    write_node_style(&root, pd, &mut output);
    write_modifiers_macro(
        &non_default_root_modifiers(&explicit_block_mods(pd, root.id())),
        &mut output,
    );
    if children.is_empty() {
        output.push_str(" {}\n");
    } else {
        output.push_str(" {\n");
        for child in &children {
            write_macro_node(child, 3, &labeler, pd, &mut output);
        }
        write_indent(&mut output, 2);
        output.push_str("}\n");
    }

    write_indent(&mut output, 1);
    output.push_str("}\n");
    write_selection_macro(state.selection.as_ref(), &labeler, &mut output);
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
    item: &Disp,
    indent_level: usize,
    labeler: &Labeler,
    pd: &ProjectedDoc,
    output: &mut String,
) {
    write_indent(output, indent_level);

    match item {
        Disp::Block(node) => {
            if let Some(l) = labeler.label(node.id()) {
                write!(output, "{l}: ").unwrap();
            }

            let type_name: &str = node.node_type().into();
            write!(output, "{type_name}").unwrap();

            write_node_style(node, pd, output);
            write_node_attrs_macro(&node.node(), output);
            write_modifiers_macro(&explicit_block_mods(pd, node.id()), output);
            write_node_marker_macro(node, pd, output);

            let children = display_children(node, pd);
            if children.is_empty() {
                output.push_str(" {}\n");
            } else {
                output.push_str(" {\n");
                for child in &children {
                    write_macro_node(child, indent_level + 1, labeler, pd, output);
                }
                write_indent(output, indent_level);
                output.push_str("}\n");
            }
        }
        Disp::Text {
            text,
            modifiers,
            style,
        } => {
            output.push_str("text");
            write!(output, "(\"{}\")", escape_str(text)).unwrap();
            if let Some(style_id) = style {
                write!(output, " @{style_id}").unwrap();
            }
            write_modifiers_macro(modifiers, output);
            output.push('\n');
        }
        Disp::Atom(leaf) => {
            if let Some(l) = labeler.label(leaf.dot()) {
                write!(output, "{l}: ").unwrap();
            }

            let type_name: &str = leaf.node_type().into();
            write!(output, "{type_name}").unwrap();

            write_leaf_style_macro(leaf, pd, output);
            write_node_attrs_macro(&atom_node(leaf, pd), output);
            write_modifiers_macro(&explicit_leaf_mods(pd, leaf.dot()), output);
            output.push('\n');
        }
    }
}

fn write_styles_macro(pd: &ProjectedDoc, output: &mut String) {
    let mut styles: Vec<(&String, &StyleEntry)> = pd.styles.iter().collect();
    if styles.is_empty() {
        return;
    }
    styles.sort_by(|a, b| a.0.cmp(b.0));

    write_indent(output, 2);
    output.push_str("styles {\n");
    for (id, entry) in styles {
        write_indent(output, 3);
        let name = entry.name.get();
        let mut mods: Vec<Modifier> = entry.modifiers.iter().cloned().collect();
        mods.sort_by_key(|m| m.as_type());

        if name == id && !mods.is_empty() {
            write!(output, "{id}:").unwrap();
        } else {
            write!(output, "{id}: \"{}\"", escape_str(name)).unwrap();
        }
        write_modifiers_macro(&mods, output);
        output.push('\n');
    }
    write_indent(output, 2);
    output.push_str("}\n");
}

fn write_node_style(node: &NodeView, pd: &ProjectedDoc, output: &mut String) {
    if let Some(style_id) = pd.node_styles.get(&node.id()).and_then(|o| o.as_ref()) {
        write!(output, " @{style_id}").unwrap();
    }
}

fn write_leaf_style_macro(leaf: &LeafView, pd: &ProjectedDoc, output: &mut String) {
    if let Some(style_id) = pd.node_styles.get(&leaf.dot()).and_then(|o| o.as_ref()) {
        write!(output, " @{style_id}").unwrap();
    }
}

fn write_node_marker_macro(node: &NodeView, pd: &ProjectedDoc, output: &mut String) {
    let Some(marker) = pd
        .node_markers
        .get(&node.id())
        .and_then(|o| o.as_ref())
        .filter(|m| !m.is_empty())
    else {
        return;
    };
    output.push_str(" marker(");
    let has_style = if let Some(style_id) = &marker.style {
        write!(output, "@{style_id}").unwrap();
        true
    } else {
        false
    };
    if !marker.modifiers.is_empty() {
        if has_style {
            output.push(' ');
        }
        let mut mods: Vec<Modifier> = marker.modifiers.clone();
        mods.sort_by_key(|m| m.as_type());
        output.push('[');
        for (i, m) in mods.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            write_modifier_macro(m, output);
        }
        output.push(']');
    }
    output.push(')');
}

fn write_selection_macro(
    selection: Option<&editor_state::Selection>,
    labeler: &Labeler,
    output: &mut String,
) {
    write_indent(output, 1);

    let Some(sel) = selection else {
        output.push_str("selection: none\n");
        return;
    };

    let show_affinity =
        sel.anchor.affinity != Affinity::Downstream || sel.head.affinity != Affinity::Downstream;

    output.push_str("selection: (");
    write_position_macro(&sel.anchor, show_affinity, labeler, output);
    output.push(')');

    if !sel.is_collapsed() {
        output.push_str(" -> (");
        write_position_macro(&sel.head, show_affinity, labeler, output);
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
    match labeler.label(pos.node) {
        Some(l) => write!(output, "{l}").unwrap(),
        None => write!(output, "{}", pos.node).unwrap(),
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
            PendingModifier::Set { modifier } => write_modifier_macro(modifier, output),
            PendingModifier::Unset { ty } => {
                let name: &str = (*ty).into();
                write!(output, "!{name}").unwrap();
            }
        }
    }
    output.push_str("]\n");
}

fn write_node_attrs_macro(node: &Node, output: &mut String) {
    let mut attrs = Vec::new();
    match node {
        Node::Blockquote(bq) if *bq.variant.get() != BlockquoteVariant::default() => {
            attrs.push(format!(
                "variant: BlockquoteVariant::{:?}",
                bq.variant.get()
            ));
        }
        Node::Callout(c) if *c.variant.get() != CalloutVariant::default() => {
            attrs.push(format!("variant: CalloutVariant::{:?}", c.variant.get()));
        }
        Node::HorizontalRule(hr) if *hr.variant.get() != HorizontalRuleVariant::default() => {
            attrs.push(format!(
                "variant: HorizontalRuleVariant::{:?}",
                hr.variant.get()
            ));
        }
        Node::Table(t) => {
            if *t.border_style.get() != TableBorderStyle::default() {
                attrs.push(format!(
                    "border_style: TableBorderStyle::{:?}",
                    t.border_style.get()
                ));
            }
            if *t.proportion.get() != 100 {
                attrs.push(format!("proportion: {}", *t.proportion.get()));
            }
        }
        Node::TableCell(tc) => {
            if let Some(w) = tc.col_width.get() {
                attrs.push(format!("col_width: Some({w})"));
            }
        }
        Node::Image(img) => {
            if let Some(id) = img.id.get() {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
            if *img.proportion.get() != 100 {
                attrs.push(format!("proportion: {}", *img.proportion.get()));
            }
        }
        Node::File(f) => {
            if let Some(id) = f.id.get() {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
        }
        Node::Embed(e) => {
            if let Some(id) = e.id.get() {
                attrs.push(format!("id: Some(\"{id}\".to_string())"));
            }
        }
        Node::Archived(a) => {
            if let Some(id) = a.id.get() {
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
        Modifier::Alignment { value } => {
            write!(output, "{name}(Alignment::{value:?})").unwrap();
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

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::inspect_state_as_macro;

    #[test]
    fn simple_state() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let output = inspect_state_as_macro(&state);
        let expected = "\
state! {
    doc {
        root {
            p1: paragraph {
                text(\"Hello\")
            }
        }
    }
    selection: (p1, 3)
}
";
        assert_eq!(output, expected);
    }

    #[test]
    fn range_selection() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        text("World")
                    }
                }
            }
            selection: (p1, 0) -> (p1, 8)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (p1, 0) -> (p1, 8)"));
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
            doc { root { p1: paragraph { text("He said \"hi\"\nnewline") } } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains(r#"text("He said \"hi\"\nnewline")"#));
    }

    #[test]
    fn modifiers_output() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold, italic] } } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("text(\"Hello\") [bold, italic]"));
    }

    #[test]
    fn non_default_paragraph_align() {
        let (state, ..) = state! {
            doc { root { p1: paragraph [alignment(Alignment::Center)] { text("Hi") } } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("[alignment(Alignment::Center)]"));
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
                    p1: paragraph {
                        text("Hello")
                        text("World")
                    }
                }
            }
            selection: (p1, 0) -> (p1, 8)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (p1, 0) -> (p1, 8)"));
        assert!(!output.contains(", >"));
        assert!(!output.contains(", <"));
    }

    #[test]
    fn affinity_shown_when_non_default() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        text("World")
                    }
                }
            }
            selection: (p1, 0, <) -> (p1, 8, >)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("selection: (p1, 0, <) -> (p1, 8, >)"));
    }

    #[test]
    fn root_default_modifiers_omitted() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
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
            doc { root [font_size(1600)] { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("root [font_size(1600)]"));
    }

    #[test]
    fn macro_output_for_none_selection() {
        use editor_macros::state;
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("selection: none"),
            "expected `selection: none` in {output}"
        );
    }

    #[test]
    fn pending_modifiers() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
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
                    p1: paragraph {
                        text("Click") [link(href: "https://example.com".to_string())]
                    }
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("text(\"Click\") [link(href: \"https://example.com\".to_string())]")
        );
    }

    #[test]
    fn styles_block_and_node_reference() {
        let (state, ..) = state! {
            doc {
                styles {
                    heading: "제목 1" [bold, font_size(2400)]
                    body: [italic]
                }
                root {
                    p1: paragraph @heading {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains(
                "        styles {\n            body: [italic]\n            heading: \"제목 1\" [bold, font_size(2400)]\n        }\n"
            ),
            "got:\n{output}"
        );
        assert!(output.contains("paragraph @heading {"), "got:\n{output}");
    }

    #[test]
    fn marker_with_style_and_modifiers() {
        let (state, ..) = state! {
            doc {
                styles {
                    s1: [italic]
                }
                root {
                    p1: paragraph marker(@s1 [bold]) {}
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("paragraph marker(@s1 [bold]) {}"),
            "got:\n{output}"
        );
    }

    #[test]
    fn marker_with_modifiers_only() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph marker([italic]) {}
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("paragraph marker([italic]) {}"),
            "got:\n{output}"
        );
    }

    #[test]
    fn marker_with_style_only() {
        let (state, ..) = state! {
            doc {
                styles {
                    s1: [italic]
                }
                root {
                    p1: paragraph marker(@s1) {}
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("paragraph marker(@s1) {}"),
            "got:\n{output}"
        );
    }

    #[test]
    fn marker_omitted_when_absent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(!output.contains("marker("), "got:\n{output}");
    }

    #[test]
    fn style_reference_on_text_and_leaf() {
        let (state, ..) = state! {
            doc {
                styles {
                    emph: [italic]
                }
                root {
                    p1: paragraph {
                        text("hi") @emph
                    }
                    horizontal_rule @emph
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("text(\"hi\") @emph"), "got:\n{output}");
        assert!(output.contains("horizontal_rule @emph\n"), "got:\n{output}");
    }
}
