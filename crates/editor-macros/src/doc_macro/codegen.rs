use heck::ToPascalCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::doc_macro::parse::{
    DecorationDef, DecorationParams, DocTree, FieldValue, NodeContent, NodeDef,
};

pub struct CodegenParts {
    pub id_decls: Vec<TokenStream>,
    pub with_nodes: Vec<TokenStream>,
    pub bindings: Vec<Ident>,
}

pub fn generate_parts(tree: &DocTree) -> CodegenParts {
    let mut id_decls = Vec::new();
    let mut with_nodes = Vec::new();
    let mut bindings = Vec::new();

    collect_node(
        &tree.root,
        &quote! {},
        false,
        &mut id_decls,
        &mut with_nodes,
        &mut bindings,
    );

    CodegenParts {
        id_decls,
        with_nodes,
        bindings,
    }
}

pub fn generate(tree: &DocTree) -> TokenStream {
    let parts = generate_parts(tree);
    let id_decls = &parts.id_decls;
    let with_nodes = &parts.with_nodes;
    let bindings = &parts.bindings;

    quote! {
        {
            use editor_model::*;

            #(#id_decls)*

            let doc = Doc::new_test();
            #(#with_nodes)*

            (doc, #(#bindings),*)
        }
    }
}

fn collect_node(
    node: &NodeDef,
    parent_id: &TokenStream,
    has_parent: bool,
    id_decls: &mut Vec<TokenStream>,
    with_nodes: &mut Vec<TokenStream>,
    bindings: &mut Vec<Ident>,
) -> TokenStream {
    let is_root = node.node_type == "root";

    let id_ident = if let Some(ref binding) = node.binding {
        binding.clone()
    } else {
        format_ident!("__node_{}", id_decls.len())
    };

    if is_root {
        id_decls.push(quote! { let #id_ident = NodeId::ROOT; });
    } else {
        id_decls.push(quote! { let #id_ident = NodeId::new(); });
    }

    if node.binding.is_some() {
        bindings.push(id_ident.clone());
    }

    let id_ts = quote! { #id_ident };

    let mut child_ids = Vec::new();

    match &node.content {
        NodeContent::Children(children) => {
            for child in children {
                let child_id = collect_node(child, &id_ts, true, id_decls, with_nodes, bindings);
                child_ids.push(child_id);
            }
        }
        NodeContent::Text(_) | NodeContent::Leaf => {}
    }

    let children_vec = quote! { editor_model::imbl::vector![#(#child_ids),*] };

    let parent_expr = if has_parent {
        quote! { Some(#parent_id) }
    } else {
        quote! { None }
    };

    let node_expr = build_node_expr(node);

    let modifiers_expr = match &node.modifiers {
        None if is_root => quote! { default_modifiers() },
        None => quote! { vec![] },
        Some(mods) if is_root => {
            let modifier_exprs: Vec<TokenStream> = mods.iter().map(build_modifier_expr).collect();
            if modifier_exprs.is_empty() {
                quote! { vec![] }
            } else {
                quote! { default_modifiers_with(vec![#(#modifier_exprs),*]) }
            }
        }
        Some(mods) => {
            let modifier_exprs: Vec<TokenStream> = mods.iter().map(build_modifier_expr).collect();
            build_entry_modifiers_expr(&modifier_exprs)
        }
    };

    let with_node = quote! {
        let doc = doc.with_node(
            #id_ident,
            NodeEntry {
                node: #node_expr,
                parent: #parent_expr,
                children: #children_vec,
                modifiers: #modifiers_expr,
            },
        );
    };
    with_nodes.push(with_node);

    id_ts
}

fn build_node_expr(node: &NodeDef) -> TokenStream {
    let type_str = node.node_type.to_string();

    if type_str == "text" {
        let text = match &node.content {
            NodeContent::Text(lit) => lit,
            _ => unreachable!("text node must have Text content"),
        };
        quote! { Node::Text(TextNode { text: #text.into() }) }
    } else {
        let variant = Ident::new(&type_str.to_pascal_case(), Span::call_site());
        let node_struct = format_ident!("{}Node", variant);

        if node.params.is_empty() {
            quote! { Node::#variant(#node_struct::default()) }
        } else {
            let field_assigns = build_field_assigns(&node.params);
            quote! {
                Node::#variant(#node_struct {
                    #(#field_assigns,)*
                    ..Default::default()
                })
            }
        }
    }
}

pub(crate) fn build_modifier_expr(dec: &DecorationDef) -> TokenStream {
    let type_str = dec.name.to_string();
    let variant = Ident::new(&type_str.to_pascal_case(), Span::call_site());

    match &dec.params {
        DecorationParams::None => quote! { Modifier::#variant },
        DecorationParams::Named(fields) => {
            let field_assigns = build_field_assigns(fields);
            quote! { Modifier::#variant { #(#field_assigns,)* } }
        }
        DecorationParams::Positional(exprs) => {
            quote! { Modifier::#variant(#(#exprs),*) }
        }
    }
}

fn build_field_assigns(fields: &[FieldValue]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|fv| {
            let name = &fv.name;
            let value = &fv.value;
            quote! { #name: #value }
        })
        .collect()
}

fn build_entry_modifiers_expr(shorthand_exprs: &[TokenStream]) -> TokenStream {
    if shorthand_exprs.is_empty() {
        quote! { vec![] }
    } else {
        quote! { vec![#(#shorthand_exprs),*] }
    }
}
