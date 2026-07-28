use syn::braced;
use syn::parse::{Parse, ParseStream};
use syn::{LitInt, Result, Token};

use crate::doc_macro::parse::{
    DecorationDef, DecorationParams, NodeContent, NodeDef, parse_node_list,
};

mod kw {
    syn::custom_keyword!(content);
    syn::custom_keyword!(open_start);
    syn::custom_keyword!(open_end);
}

pub struct SliceInput {
    pub content: Vec<NodeDef>,
    pub open_start: LitInt,
    pub open_end: LitInt,
}

impl Parse for SliceInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<kw::content>()?;

        let content;
        braced!(content in input);
        let content = parse_node_list(&content)?;
        validate_slice_nodes(&content)?;

        input.parse::<kw::open_start>()?;
        input.parse::<Token![:]>()?;
        let open_start = input.parse()?;

        input.parse::<kw::open_end>()?;
        input.parse::<Token![:]>()?;
        let open_end = input.parse()?;

        Ok(Self {
            content,
            open_start,
            open_end,
        })
    }
}

fn validate_slice_nodes(nodes: &[NodeDef]) -> Result<()> {
    for node in nodes {
        if let Some(binding) = &node.binding {
            return Err(syn::Error::new(
                binding.span(),
                "slice nodes cannot be labeled because slices contain plain fragments without projected identities",
            ));
        }
        if node.synthetic {
            return Err(syn::Error::new(
                node.node_type.span(),
                "slice nodes cannot be synthetic because slices contain plain fragments without projected identities",
            ));
        }
        validate_modifiers(node.modifiers.as_deref())?;
        validate_modifiers(node.carry.as_ref().map(|carry| carry.modifiers.as_slice()))?;
        if let NodeContent::Children(children) = &node.content {
            validate_slice_nodes(children)?;
        }
    }
    Ok(())
}

fn validate_modifiers(modifiers: Option<&[DecorationDef]>) -> Result<()> {
    for modifier in modifiers.into_iter().flatten() {
        if let DecorationParams::Positional(params) = &modifier.params
            && params.len() != 1
        {
            return Err(syn::Error::new(
                modifier.name.span(),
                "positional modifier shorthand expects exactly one argument",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SliceInput;

    fn assert_parse_error(input: &str, expected_message: &str) {
        let error = syn::parse_str::<SliceInput>(input)
            .err()
            .expect("slice input must be rejected");
        assert!(
            error.to_string().contains(expected_message),
            "expected error containing {expected_message:?}, got {error}"
        );
    }

    #[test]
    fn positional_modifier_requires_exactly_one_argument() {
        let input = r#"
            content {
                paragraph [font_size(1, 2)] {}
            }
            open_start: 0
            open_end: 0
        "#;

        assert_parse_error(
            input,
            "positional modifier shorthand expects exactly one argument",
        );
    }

    #[test]
    fn nested_block_labels_are_rejected() {
        let input = r#"
            content {
                paragraph {
                    child: paragraph {}
                }
            }
            open_start: 0
            open_end: 0
        "#;

        assert_parse_error(input, "slice nodes cannot be labeled");
    }

    #[test]
    fn nested_synthetic_nodes_are_rejected() {
        let input = r#"
            content {
                paragraph {
                    synthetic paragraph {}
                }
            }
            open_start: 0
            open_end: 0
        "#;

        assert_parse_error(input, "slice nodes cannot be synthetic");
    }

    #[test]
    fn slice_fields_must_follow_the_required_order() {
        let input = r#"
            content {}
            open_end: 0
            open_start: 0
        "#;

        assert_parse_error(input, "expected `open_start`");
    }

    #[test]
    fn slice_requires_open_end() {
        let input = r#"
            content {}
            open_start: 0
        "#;

        assert_parse_error(input, "expected `open_end`");
    }
}
