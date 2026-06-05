use editor_macros::ffi;
use editor_model::*;
use editor_state::{Affinity, State};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::labeler::Labeler;

#[ffi]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectStateOptions {
    pub show_node_ids: bool,
}

pub fn inspect_state(state: &State, options: &InspectStateOptions) -> String {
    let labeler = Labeler::new(&state.doc, state.selection.as_ref());
    let mut output = String::new();

    write_styles_tree(&state.doc, &mut output);

    let root = state.doc.root().unwrap();
    if let Some(l) = labeler.label(root.id()) {
        write!(output, "{l}: ").unwrap();
    }
    output.push_str("root");
    if options.show_node_ids {
        write!(output, " ({})", root.id()).unwrap();
    }
    write_node_style_tree(&root, &mut output);
    let mut root_mods: Vec<Modifier> = root.explicit_modifiers().cloned().collect();
    root_mods.sort_by_key(|m| m.as_type());
    write_modifiers_tree(&non_default_root_modifiers(&root_mods), &mut output);
    output.push('\n');

    let children: Vec<_> = root.children().collect();
    for (i, child) in children.iter().enumerate() {
        write_tree_node(
            child,
            "",
            i == children.len() - 1,
            &labeler,
            options,
            &mut output,
        );
    }

    output.push('\n');
    write_selection_tree(state.selection.as_ref(), &labeler, &mut output);

    output
}

fn write_tree_node(
    node_ref: &NodeRef,
    prefix: &str,
    is_last: bool,
    labeler: &Labeler,
    options: &InspectStateOptions,
    output: &mut String,
) {
    let connector = if is_last { "└─ " } else { "├─ " };
    write!(output, "{prefix}{connector}").unwrap();

    if let Some(l) = labeler.label(node_ref.id()) {
        write!(output, "{l}: ").unwrap();
    }

    let type_name: &str = node_ref.as_type().into();
    write!(output, "{type_name}").unwrap();

    if options.show_node_ids {
        write!(output, " ({})", node_ref.id()).unwrap();
    }

    if let Node::Text(t) = node_ref.node() {
        let s = t.text.to_string();
        write!(output, " \"{}\"", truncate_text(&s, 50)).unwrap();
    }

    write_node_style_tree(node_ref, output);
    write_node_attrs_tree(node_ref.node(), output);
    let mut mods: Vec<Modifier> = node_ref.explicit_modifiers().cloned().collect();
    mods.sort_by_key(|m| m.as_type());
    write_modifiers_tree(&mods, output);
    output.push('\n');

    let child_prefix = if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}│  ")
    };
    let children: Vec<_> = node_ref.children().collect();
    for (i, child) in children.iter().enumerate() {
        write_tree_node(
            child,
            &child_prefix,
            i == children.len() - 1,
            labeler,
            options,
            output,
        );
    }
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
    match labeler.label(pos.node_id) {
        Some(l) => write!(output, "{l}").unwrap(),
        None => write!(output, "{}", pos.node_id).unwrap(),
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

fn write_styles_tree(doc: &Doc, output: &mut String) {
    let mut styles: Vec<(&String, &StyleEntry)> = doc
        .styles_iter()
        .filter_map(|(id, _)| doc.style_entry(id).map(|entry| (id, entry)))
        .collect();
    if styles.is_empty() {
        return;
    }
    styles.sort_by(|a, b| a.0.cmp(b.0));

    output.push_str("styles:\n");
    for (id, entry) in styles {
        let name = entry.name.get();
        write!(output, "  {id}").unwrap();
        if name != id {
            write!(output, " \"{name}\"").unwrap();
        }
        let mut mods: Vec<Modifier> = entry.modifiers.iter().cloned().collect();
        mods.sort_by_key(|m| m.as_type());
        write_modifiers_tree(&mods, output);
        output.push('\n');
    }
    output.push('\n');
}

fn write_node_style_tree(node_ref: &NodeRef, output: &mut String) {
    if let Some(style_id) = node_ref.entry().style.get() {
        write!(output, " style=\"{style_id}\"").unwrap();
    }
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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        let expected = "\
root
└─ paragraph
   └─ t1: text \"Hello\"

selection: (t1, 0, >)
";
        assert_eq!(output, expected);
    }

    #[test]
    fn multiple_paragraphs() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let output = inspect_state(&state, &opts());
        let expected = "\
root
├─ paragraph
│  └─ t1: text \"A\"
└─ paragraph
   └─ t2: text \"B\"

selection: (t1, 0, >) -> (t2, 1, >)
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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("selection: (t1, 3, >)"));
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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(!para_line.contains("align"));
    }

    #[test]
    fn paragraph_shows_non_default_align() {
        let (state, ..) = state! {
            doc { root { paragraph [alignment(Alignment::Center)] { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("[alignment(Center)]"));
    }

    #[test]
    fn text_with_modifiers() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold, italic] } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("text \"Hello\" [bold, italic]"));
    }

    #[test]
    fn show_node_ids() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let options = InspectStateOptions {
            show_node_ids: true,
        };
        let output = inspect_state(&state, &options);
        assert!(output.starts_with("root (0)"));
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(para_line.contains("("));
    }

    #[test]
    fn root_default_modifiers_hidden() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        let root_line = output.lines().next().unwrap();
        assert_eq!(root_line, "root");
    }

    #[test]
    fn root_non_default_modifiers_shown() {
        let (state, ..) = state! {
            doc { root [font_size(1600)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.starts_with("root [font_size(1600)]"));
    }

    #[test]
    fn text_truncation() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA") } } }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(output.contains("...\""));
    }

    #[test]
    fn styles_section_and_node_style() {
        let (state, ..) = state! {
            doc {
                styles {
                    heading: "제목 1" [bold, font_size(2400)]
                    body: [italic]
                }
                root {
                    paragraph @heading {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        assert!(
            output.starts_with(
                "styles:\n  body [italic]\n  heading \"제목 1\" [bold, font_size(2400)]\n\nroot\n"
            ),
            "got:\n{output}"
        );
        let para_line = output.lines().find(|l| l.contains("paragraph")).unwrap();
        assert!(para_line.contains("style=\"heading\""), "got:\n{output}");
    }

    #[test]
    fn node_style_on_text_and_leaf() {
        let (state, ..) = state! {
            doc {
                styles { emph: [italic] }
                root {
                    paragraph {
                        t1: text("hi") @emph
                    }
                    horizontal_rule @emph
                }
            }
            selection: (t1, 0)
        };
        let output = inspect_state(&state, &opts());
        let text_line = output.lines().find(|l| l.contains("text")).unwrap();
        assert!(text_line.contains("style=\"emph\""), "got:\n{output}");
        let hr_line = output
            .lines()
            .find(|l| l.contains("horizontal_rule"))
            .unwrap();
        assert!(hr_line.contains("style=\"emph\""), "got:\n{output}");
    }
}
