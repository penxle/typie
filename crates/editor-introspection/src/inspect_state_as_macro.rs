use editor_model::*;
use editor_state::{Affinity, PendingModifier, State};
use std::collections::BTreeMap;
use std::fmt::Write;

use crate::labeler::Labeler;
use crate::macro_format::{
    escape_str, write_carry_macro, write_indent, write_modifier_macro, write_modifiers_macro,
    write_node_attrs_macro,
};

enum Disp<'a> {
    Block(NodeView<'a>),
    Text {
        text: String,
        modifiers: Vec<Modifier>,
    },
    Atom {
        leaf: LeafView<'a>,
        modifiers: Vec<Modifier>,
    },
}

fn display_children<'a>(node: &NodeView<'a>) -> Vec<Disp<'a>> {
    let mut out: Vec<Disp<'a>> = Vec::new();
    let mut run: Option<(String, Vec<Modifier>)> = None;
    for (slot, child) in node.children().enumerate() {
        match child {
            ChildView::Block(b) => {
                if let Some((text, modifiers)) = run.take() {
                    out.push(Disp::Text { text, modifiers });
                }
                out.push(Disp::Block(b));
            }
            ChildView::Leaf(l) => match l.as_char() {
                Some(c) => {
                    let modifiers = node
                        .leaf_state_at(slot)
                        .map(|s| explicit_leaf_mods(s.own))
                        .unwrap_or_default();
                    let extend = matches!(&run, Some((_, m)) if *m == modifiers);
                    if extend {
                        if let Some((text, _)) = run.as_mut() {
                            text.push(c);
                        }
                    } else {
                        if let Some((text, modifiers)) = run.take() {
                            out.push(Disp::Text { text, modifiers });
                        }
                        run = Some((c.to_string(), modifiers));
                    }
                }
                None => {
                    if let Some((text, modifiers)) = run.take() {
                        out.push(Disp::Text { text, modifiers });
                    }
                    let modifiers = node
                        .leaf_state_at(slot)
                        .map(|s| explicit_leaf_mods(s.own))
                        .unwrap_or_default();
                    out.push(Disp::Atom { leaf: l, modifiers });
                }
            },
        }
    }
    if let Some((text, modifiers)) = run.take() {
        out.push(Disp::Text { text, modifiers });
    }
    out
}

fn atom_node(leaf: &LeafView, _pd: &ProjectedDoc) -> Node {
    leaf.node().expect("atom leaf")
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

/// Modifiers explicitly applied to a leaf via spans (excluding inherited
/// modifiers).
fn explicit_leaf_mods(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    let mut mods: Vec<Modifier> = own.values().map(|o| o.value.clone()).collect();
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

    let root = view.root().unwrap();
    let children = display_children(&root);

    write_indent(&mut output, 2);
    if let Some(l) = labeler.label(root.id()) {
        write!(output, "{l}: ").unwrap();
    }
    output.push_str("root");
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

            if is_synthetic_scaffold(node.id()) {
                output.push_str("synthetic ");
            }

            let type_name: &str = node.node_type().into();
            write!(output, "{type_name}").unwrap();

            write_node_attrs_macro(&node.node().to_plain(), output);
            write_modifiers_macro(&explicit_block_mods(pd, node.id()), output);
            write_node_carry_macro(node, pd, output);

            let children = display_children(node);
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
        Disp::Text { text, modifiers } => {
            output.push_str("text");
            write!(output, "(\"{}\")", escape_str(text)).unwrap();
            write_modifiers_macro(modifiers, output);
            output.push('\n');
        }
        Disp::Atom { leaf, modifiers } => {
            if let Some(l) = labeler.label(leaf.dot()) {
                write!(output, "{l}: ").unwrap();
            }

            let type_name: &str = leaf.node_type().into();
            write!(output, "{type_name}").unwrap();

            write_node_attrs_macro(&atom_node(leaf, pd).to_plain(), output);
            write_modifiers_macro(modifiers, output);
            output.push('\n');
        }
    }
}

fn is_synthetic_scaffold(id: editor_crdt::Dot) -> bool {
    id.is_synthetic() && id != editor_crdt::Dot::ROOT
}

fn write_node_carry_macro(node: &NodeView, pd: &ProjectedDoc, output: &mut String) {
    let carry: Vec<_> = pd.carry_modifiers(node.id()).into_values().collect();
    write_carry_macro(&carry, output);
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
    fn synthetic_scaffold_is_marked() {
        let (state, ..) = state! {
            doc { root { horizontal_rule } }
            selection: none
        };
        let output = inspect_state_as_macro(&state);
        assert!(output.contains("synthetic paragraph {}"), "got:\n{output}");
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
    fn carry_with_modifiers_only() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([italic]) {}
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(
            output.contains("paragraph carry([italic]) {}"),
            "got:\n{output}"
        );
    }

    #[test]
    fn carry_omitted_when_absent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let output = inspect_state_as_macro(&state);
        assert!(!output.contains("carry("), "got:\n{output}");
    }

    #[test]
    fn non_carry_kind_in_carry_log_is_not_echoed() {
        use editor_model::{EditOp, Modifier, ModifierAttrOp};

        let (mut state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        std::sync::Arc::make_mut(&mut state.projected)
            .apply(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target: p1,
                modifier: Modifier::LineHeight { value: 200 },
            }))
            .unwrap();
        let output = inspect_state_as_macro(&state);
        assert!(
            !output.contains("carry("),
            "a non-carry kind in the carry log never reaches the inspect output, got:\n{output}"
        );
    }
}
