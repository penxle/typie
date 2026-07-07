use editor_macros::ffi;
use editor_model::*;
use editor_state::{Affinity, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write;

use crate::labeler::Labeler;

#[ffi]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectStateOptions {
    pub show_node_ids: bool,
}

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

/// Modifiers explicitly set on a block (via `SetModifier`), excluding inherited
/// and schema-default modifiers — the persisted-only view the inspector and the
/// round-trip macro both want.
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
/// modifiers), from the leaf's run-segment own map.
fn explicit_leaf_mods(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    let mut mods: Vec<Modifier> = own.values().map(|o| o.value.clone()).collect();
    mods.sort_by_key(|m| m.as_type());
    mods
}

pub fn inspect_state(state: &State, options: &InspectStateOptions) -> String {
    let view = state.view();
    let pd = state.projected.projected();
    let labeler = Labeler::new(&view, state.selection.as_ref());
    let mut output = String::new();

    let root = view.root().unwrap();
    if let Some(l) = labeler.label(root.id()) {
        write!(output, "{l}: ").unwrap();
    }
    output.push_str("root");
    if options.show_node_ids {
        write!(output, " ({})", root.id()).unwrap();
    }
    // The macro/builder seeds the root with the full schema-default modifier set
    // (so font resolution always finds a family/weight); only surface overrides.
    write_modifiers_tree(
        &non_default_root_modifiers(&explicit_block_mods(pd, root.id())),
        &mut output,
    );
    output.push('\n');

    let children = display_children(&root);
    for (i, child) in children.iter().enumerate() {
        write_tree_node(
            child,
            "",
            i == children.len() - 1,
            &labeler,
            options,
            pd,
            &mut output,
        );
    }

    output.push('\n');
    write_selection_tree(state.selection.as_ref(), &labeler, &mut output);

    output
}

fn write_tree_node(
    item: &Disp,
    prefix: &str,
    is_last: bool,
    labeler: &Labeler,
    options: &InspectStateOptions,
    pd: &ProjectedDoc,
    output: &mut String,
) {
    let connector = if is_last { "└─ " } else { "├─ " };
    write!(output, "{prefix}{connector}").unwrap();

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

            if options.show_node_ids {
                write!(output, " ({})", node.id()).unwrap();
            }

            write_node_attrs_tree(&node.node(), output);
            write_modifiers_tree(&explicit_block_mods(pd, node.id()), output);
            write_node_carry_tree(node, pd, output);
            output.push('\n');

            let child_prefix = if is_last {
                format!("{prefix}   ")
            } else {
                format!("{prefix}│  ")
            };
            let children = display_children(node);
            for (i, child) in children.iter().enumerate() {
                write_tree_node(
                    child,
                    &child_prefix,
                    i == children.len() - 1,
                    labeler,
                    options,
                    pd,
                    output,
                );
            }
        }
        Disp::Text { text, modifiers } => {
            output.push_str("text");
            write!(output, " \"{}\"", truncate_text(text, 50)).unwrap();
            write_modifiers_tree(modifiers, output);
            output.push('\n');
        }
        Disp::Atom { leaf, modifiers } => {
            if let Some(l) = labeler.label(leaf.dot()) {
                write!(output, "{l}: ").unwrap();
            }

            let type_name: &str = leaf.node_type().into();
            write!(output, "{type_name}").unwrap();

            if options.show_node_ids {
                write!(output, " ({})", leaf.dot()).unwrap();
            }

            write_node_attrs_tree(&atom_node(leaf, pd), output);
            write_modifiers_tree(modifiers, output);
            output.push('\n');
        }
    }
}

fn is_synthetic_scaffold(id: editor_crdt::Dot) -> bool {
    id.is_synthetic() && id != editor_crdt::Dot::ROOT
}

fn write_selection_tree(
    selection: Option<&editor_state::Selection>,
    labeler: &Labeler,
    output: &mut String,
) {
    let Some(sel) = selection else {
        output.push_str("selection: <none>\n");
        return;
    };

    output.push_str("selection: (");
    write_position_tree(&sel.anchor, labeler, output);
    output.push(')');

    if !sel.is_collapsed() {
        output.push_str(" -> (");
        write_position_tree(&sel.head, labeler, output);
        output.push(')');
    }
    output.push('\n');
}

fn write_position_tree(pos: &editor_state::Position, labeler: &Labeler, output: &mut String) {
    match labeler.label(pos.node) {
        Some(l) => write!(output, "{l}").unwrap(),
        None => write!(output, "{}", pos.node).unwrap(),
    }
    let aff = match pos.affinity {
        Affinity::Downstream => ">",
        Affinity::Upstream => "<",
    };
    write!(output, ", {}, {aff}", pos.offset).unwrap();
}

fn write_node_attrs_tree(node: &Node, output: &mut String) {
    match node {
        Node::Blockquote(bq) => {
            write!(output, " variant={:?}", bq.variant.get()).unwrap();
        }
        Node::Callout(c) => {
            write!(output, " variant={:?}", c.variant.get()).unwrap();
        }
        Node::HorizontalRule(hr) => {
            write!(output, " variant={:?}", hr.variant.get()).unwrap();
        }
        Node::Table(t) => {
            write!(output, " border_style={:?}", t.border_style.get()).unwrap();
            if *t.proportion.get() != 100 {
                write!(output, " proportion={}", t.proportion.get()).unwrap();
            }
        }
        Node::TableCell(tc) => {
            if let Some(w) = tc.col_width.get() {
                write!(output, " col_width={w}").unwrap();
            }
        }
        Node::Image(img) => {
            if let Some(id) = img.id.get() {
                write!(output, " id=\"{id}\"").unwrap();
            }
            if *img.proportion.get() != 100 {
                write!(output, " proportion={}", img.proportion.get()).unwrap();
            }
        }
        Node::File(f) => {
            if let Some(id) = f.id.get() {
                write!(output, " id=\"{id}\"").unwrap();
            }
        }
        Node::Embed(e) => {
            if let Some(id) = e.id.get() {
                write!(output, " id=\"{id}\"").unwrap();
            }
        }
        Node::Archived(a) => {
            if let Some(id) = a.id.get() {
                write!(output, " id=\"{id}\"").unwrap();
            }
        }
        _ => {}
    }
}

fn write_node_carry_tree(node: &NodeView, pd: &ProjectedDoc, output: &mut String) {
    let carry = pd.carry_modifiers(node.id());
    if carry.is_empty() {
        return;
    }
    output.push_str(" carry=");
    output.push('[');
    for (i, m) in carry.values().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_modifier_tree(m, output);
    }
    output.push(']');
}

fn write_modifier_tree(m: &Modifier, output: &mut String) {
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
            write!(output, "{name}(\"{value}\")").unwrap();
        }
        Modifier::Link { href } => write!(output, "{name}(href: \"{href}\")").unwrap(),
        Modifier::Ruby { text } => write!(output, "{name}(text: \"{text}\")").unwrap(),
        Modifier::Alignment { value } => write!(output, "{name}({value:?})").unwrap(),
    }
}

fn write_modifiers_tree(modifiers: &[Modifier], output: &mut String) {
    if modifiers.is_empty() {
        return;
    }
    output.push_str(" [");
    for (i, m) in modifiers.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_modifier_tree(m, output);
    }
    output.push(']');
}

fn non_default_root_modifiers(modifiers: &[Modifier]) -> Vec<Modifier> {
    let defaults = editor_model::default_modifiers();
    modifiers
        .iter()
        .filter(|m| !defaults.contains(m))
        .cloned()
        .collect()
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    fn opts() -> InspectStateOptions {
        InspectStateOptions {
            show_node_ids: false,
        }
    }

    #[test]
    fn none_selection() {
        use editor_macros::state;
        let (state, ..) = state! {
            doc { root { paragraph { text("hello") } } }
            selection: none
        };
        let output = inspect_state(&state, &opts());
        assert!(
            output.contains("selection: <none>"),
            "expected `selection: <none>` in {output}"
        );
    }

    #[test]
    fn simple_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        let expected = "\
root
└─ p1: paragraph
   └─ text \"Hello\"

selection: (p1, 0, >)
";
        assert_eq!(output, expected);
    }

    #[test]
    fn multiple_paragraphs() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let output = inspect_state(&state, &opts());
        let expected = "\
root
├─ p1: paragraph
│  └─ text \"A\"
└─ p2: paragraph
   └─ text \"B\"

selection: (p1, 0, >) -> (p2, 1, >)
";
        assert_eq!(output, expected);
    }

    #[test]
    fn empty_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        let expected = "\
root
└─ p1: paragraph

selection: (p1, 0, >)
";
        assert_eq!(output, expected);
    }

    #[test]
    fn collapsed_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("selection: (p1, 3, >)"));
    }

    #[test]
    fn blockquote_always_shows_variant() {
        let (state, ..) = state! {
            doc { root { blockquote { p1: paragraph {} } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("blockquote variant=LeftLine"));
    }

    #[test]
    fn paragraph_hides_default_align() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(!para_line.contains("align"));
    }

    #[test]
    fn paragraph_shows_non_default_align() {
        let (state, ..) = state! {
            doc { root { p1: paragraph [alignment(Alignment::Center)] { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("[alignment(Center)]"));
    }

    #[test]
    fn text_with_modifiers() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold, italic] } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("text \"Hello\" [bold, italic]"));
    }

    #[test]
    fn show_node_ids() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let options = InspectStateOptions {
            show_node_ids: true,
        };
        let output = inspect_state(&state, &options);
        assert!(output.starts_with("root ("));
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(para_line.contains("("));
    }

    #[test]
    fn synthetic_scaffold_is_marked() {
        let (state, ..) = state! {
            doc { root { horizontal_rule } }
            selection: none
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("synthetic paragraph"), "got:\n{output}");
    }

    #[test]
    fn root_default_modifiers_hidden() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        let root_line = output.lines().next().unwrap();
        assert_eq!(root_line, "root");
    }

    #[test]
    fn root_non_default_modifiers_shown() {
        let (state, ..) = state! {
            doc { root [font_size(1600)] { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.starts_with("root [font_size(1600)]"));
    }

    #[test]
    fn text_truncation() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA") } } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("...\""));
    }

    #[test]
    fn carry_modifiers_shown() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([bold]) {}
                }
            }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(para_line.contains("carry="), "got:\n{output}");
        assert!(para_line.contains("[bold]"), "got:\n{output}");
    }

    #[test]
    fn carry_omitted_when_absent() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(!output.contains("carry="), "got:\n{output}");
    }
}
