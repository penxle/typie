use proc_macro2::TokenStream;
use quote::quote;

use crate::doc_macro::codegen::{build_modifier_expr, build_plain_node_expr};
use crate::doc_macro::parse::{CarryDef, DecorationDef, NodeContent, NodeDef};
use crate::slice_macro::parse::SliceInput;

pub fn generate(input: &SliceInput) -> TokenStream {
    let content = input.content.iter().map(generate_fragment);
    let open_start = &input.open_start;
    let open_end = &input.open_end;

    quote! {
        {
            use ::editor_model::*;

            ::editor_clipboard::Slice::new(
                vec![#(#content),*],
                #open_start,
                #open_end,
            )
        }
    }
}

fn generate_fragment(node: &NodeDef) -> TokenStream {
    let node_expr = match &node.content {
        NodeContent::Text(text) => {
            quote! { PlainNode::Text(PlainTextNode { text: #text.to_string() }) }
        }
        NodeContent::Children(_) | NodeContent::Leaf => build_plain_node_expr(node),
    };
    let modifiers = generate_modifiers(node.modifiers.as_deref());
    let carry = generate_carry(node.carry.as_ref());
    let children = match &node.content {
        NodeContent::Children(children) => {
            let children = children.iter().map(generate_fragment);
            quote! { vec![#(#children),*] }
        }
        NodeContent::Text(_) | NodeContent::Leaf => quote! { Vec::new() },
    };

    quote! {
        ::editor_model::Fragment {
            node: #node_expr,
            modifiers: #modifiers,
            carry: #carry,
            children: #children,
        }
    }
}

fn generate_modifiers(modifiers: Option<&[DecorationDef]>) -> TokenStream {
    match modifiers {
        Some(modifiers) if !modifiers.is_empty() => {
            let modifiers = modifiers.iter().map(build_modifier_expr);
            quote! { vec![#(#modifiers),*] }
        }
        Some(_) | None => quote! { Vec::new() },
    }
}

fn generate_carry(carry: Option<&CarryDef>) -> TokenStream {
    match carry {
        Some(carry) => {
            let modifiers = carry.modifiers.iter().map(build_modifier_expr);
            quote! { vec![#(#modifiers),*] }
        }
        None => quote! { Vec::new() },
    }
}
