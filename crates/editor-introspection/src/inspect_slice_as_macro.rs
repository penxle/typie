use std::fmt::Write;

use editor_clipboard::Slice;
use editor_model::{Fragment, PlainNode};
use editor_state::State;

use crate::macro_format::{
    escape_str, write_carry_macro, write_indent, write_modifiers_macro, write_node_attrs_macro,
};

pub fn inspect_selection_as_slice_macro(state: &State) -> Option<String> {
    let slice = Slice::extract(state)?;
    Some(inspect_slice_as_macro(&slice))
}

pub fn inspect_slice_as_macro(slice: &Slice) -> String {
    let mut output = String::new();

    output.push_str("slice! {\n");
    write_indent(&mut output, 1);
    if slice.content.is_empty() {
        output.push_str("content {}\n");
    } else {
        output.push_str("content {\n");
        for fragment in &slice.content {
            write_fragment(fragment, 2, &mut output);
        }
        write_indent(&mut output, 1);
        output.push_str("}\n");
    }
    write_indent(&mut output, 1);
    writeln!(output, "open_start: {}", slice.open_start).unwrap();
    write_indent(&mut output, 1);
    writeln!(output, "open_end: {}", slice.open_end).unwrap();
    output.push_str("}\n");

    output
}

fn write_fragment(fragment: &Fragment, indent_level: usize, output: &mut String) {
    write_indent(output, indent_level);

    if let PlainNode::Text(text) = &fragment.node {
        write!(output, "text(\"{}\")", escape_str(&text.text)).unwrap();
        write_modifiers_macro(&fragment.modifiers, output);
        write_carry_macro(&fragment.carry, output);
        output.push('\n');
        return;
    }

    let type_name: &str = fragment.node.as_type().into();
    output.push_str(type_name);
    write_node_attrs_macro(&fragment.node, output);
    write_modifiers_macro(&fragment.modifiers, output);
    write_carry_macro(&fragment.carry, output);

    if fragment.children.is_empty() {
        if fragment.node.as_type().spec().is_leaf() {
            output.push('\n');
        } else {
            output.push_str(" {}\n");
        }
        return;
    }

    output.push_str(" {\n");
    for child in &fragment.children {
        write_fragment(child, indent_level + 1, output);
    }
    write_indent(output, indent_level);
    output.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use editor_macros::{slice, state};

    use crate::{inspect_selection_as_slice_macro, inspect_slice_as_macro};

    #[test]
    fn collapsed_selection_has_no_slice_macro() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };

        assert_eq!(inspect_selection_as_slice_macro(&state), None);
    }

    #[test]
    fn selected_range_is_extracted_as_slice_macro() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };

        assert_eq!(
            inspect_selection_as_slice_macro(&state),
            Some(
                "\
slice! {
    content {
        text(\"ell\")
    }
    open_start: 0
    open_end: 0
}
"
                .to_string()
            )
        );
    }

    #[test]
    fn nested_multi_root_slice() {
        let slice = slice! {
            content {
                paragraph [alignment(Alignment::Center)] carry([bold]) {
                    text("He said \"hi\"\n") [italic]
                }
                image(id: Some("asset-id".to_string()), proportion: 80)
            }
            open_start: 1
            open_end: 2
        };

        let output = inspect_slice_as_macro(&slice);
        let expected = r#"slice! {
    content {
        paragraph [alignment(Alignment::Center)] carry([bold]) {
            text("He said \"hi\"\n") [italic]
        }
        image(id: Some("asset-id".to_string()), proportion: 80)
    }
    open_start: 1
    open_end: 2
}
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn empty_slice_uses_normalized_open_depths() {
        let slice = slice! {
            content {}
            open_start: 3
            open_end: 4
        };

        let output = inspect_slice_as_macro(&slice);
        let expected = "\
slice! {
    content {}
    open_start: 0
    open_end: 0
}
";

        assert_eq!(output, expected);
    }

    #[test]
    fn escapes_all_rust_string_literals() {
        let slice = slice! {
            content {
                paragraph {
                    text("styles") [
                        font_family("Font\"Family".to_string()),
                        text_color("red\\blue".to_string()),
                        background_color("line\nbreak".to_string()),
                        link(href: "tab\tlink".to_string()),
                        ruby(text: "ruby\r\u{2}".to_string()),
                    ]
                }
                image(id: Some("asset\"\\\n\r\t\u{1}".to_string()))
                file(id: Some("file\"id".to_string()))
                embed(id: Some("embed\\id".to_string()))
                archived(id: Some("archived\nid".to_string()))
            }
            open_start: 0
            open_end: 0
        };

        let output = inspect_slice_as_macro(&slice);
        let expected = r#"slice! {
    content {
        paragraph {
            text("styles") [font_family("Font\"Family".to_string()), text_color("red\\blue".to_string()), background_color("line\nbreak".to_string()), link(href: "tab\tlink".to_string()), ruby(text: "ruby\r\u{2}".to_string())]
        }
        image(id: Some("asset\"\\\n\r\t\u{1}".to_string()))
        file(id: Some("file\"id".to_string()))
        embed(id: Some("embed\\id".to_string()))
        archived(id: Some("archived\nid".to_string()))
    }
    open_start: 0
    open_end: 0
}
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn emits_all_non_default_plain_node_attributes() {
        let slice = slice! {
            content {
                root(
                    layout_mode: LayoutMode::Paginated {
                        page_width: 800,
                        page_height: 1200,
                        page_margin_top: 10,
                        page_margin_bottom: 20,
                        page_margin_left: 30,
                        page_margin_right: 40,
                    },
                ) {}
                table_cell(
                    col_width: Some(320),
                    background_color: Some("blue\"gray".to_string()),
                ) {}
            }
            open_start: 0
            open_end: 0
        };

        let output = inspect_slice_as_macro(&slice);
        let expected = r#"slice! {
    content {
        root(layout_mode: LayoutMode::Paginated { page_width: 800, page_height: 1200, page_margin_top: 10, page_margin_bottom: 20, page_margin_left: 30, page_margin_right: 40 }) {}
        table_cell(col_width: Some(320), background_color: Some("blue\"gray".to_string())) {}
    }
    open_start: 0
    open_end: 0
}
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn formats_unknown_unit_variant() {
        let slice = slice! {
            content {
                unknown {}
            }
            open_start: 0
            open_end: 0
        };

        let output = inspect_slice_as_macro(&slice);
        let expected = "\
slice! {
    content {
        unknown {}
    }
    open_start: 0
    open_end: 0
}
";

        assert_eq!(output, expected);
    }
}
