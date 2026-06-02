use heck::ToPascalCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::doc_macro::parse::{
    DecorationDef, DecorationParams, DocTree, FieldValue, NodeContent, NodeDef,
};

pub struct CodegenParts {
    pub id_decls: Vec<TokenStream>,
    pub plain_entries: Vec<TokenStream>,
    pub bindings: Vec<Ident>,
}

pub fn generate_parts(tree: &DocTree) -> CodegenParts {
    let mut id_decls = Vec::new();
    let mut plain_entries = Vec::new();
    let mut bindings = Vec::new();

    collect_node(
        &tree.root,
        None,
        &mut id_decls,
        &mut plain_entries,
        &mut bindings,
    );

    CodegenParts {
        id_decls,
        plain_entries,
        bindings,
    }
}

pub fn generate(tree: &DocTree) -> TokenStream {
    let parts = generate_parts(tree);
    let bindings = &parts.bindings;
    let scaffold = emit_doc_construction(&parts);

    quote! {
        {
            #scaffold

            (doc, #(#bindings),*)
        }
    }
}

pub(crate) fn emit_doc_construction(parts: &CodegenParts) -> TokenStream {
    let id_decls = &parts.id_decls;
    let plain_entries = &parts.plain_entries;
    quote! {
        use ::editor_model::*;
        use ::std::collections::BTreeMap;

        #(#id_decls)*

        let __plain = PlainDoc {
            nodes: {
                let mut __m: BTreeMap<NodeId, PlainNodeEntry> = BTreeMap::new();
                #(#plain_entries)*
                __m
            },
            styles: BTreeMap::new(),
        };
        let (doc, _op_graph) = Doc::from_plain(__plain);
    }
}

// Returns the TokenStream for this node's id ident (e.g. `quote! { __node_0 }`).
fn collect_node(
    node: &NodeDef,
    parent_id: Option<&TokenStream>,
    id_decls: &mut Vec<TokenStream>,
    plain_entries: &mut Vec<TokenStream>,
    bindings: &mut Vec<Ident>,
) -> TokenStream {
    let is_root = node.node_type == "root";
    let is_text = node.node_type == "text";

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

    // Collect children first (so their id_decls are in DFS order).
    let child_ids: Vec<TokenStream> = match &node.content {
        NodeContent::Children(children) => children
            .iter()
            .map(|child| collect_node(child, Some(&id_ts), id_decls, plain_entries, bindings))
            .collect(),
        NodeContent::Text(_) | NodeContent::Leaf => vec![],
    };

    let parent_expr = match parent_id {
        Some(pid) => quote! { Some(#pid) },
        None => quote! { None },
    };

    let children_expr = if child_ids.is_empty() {
        quote! { vec![] }
    } else {
        quote! { vec![#(#child_ids),*] }
    };

    let modifiers_expr = build_modifiers_expr(node);
    let plain_node_expr = build_plain_node_expr(node);

    // For text nodes, the text literal is held in the PlainNode directly.
    let plain_node_with_text = if is_text {
        if let NodeContent::Text(lit) = &node.content {
            quote! { PlainNode::Text(PlainTextNode { text: #lit.to_string() }) }
        } else {
            plain_node_expr
        }
    } else {
        plain_node_expr
    };

    plain_entries.push(quote! {
        __m.insert(#id_ident, PlainNodeEntry {
            parent: #parent_expr,
            children: #children_expr,
            modifiers: #modifiers_expr,
            style: None,
            node: #plain_node_with_text,
        });
    });

    id_ts
}

// Produces the PlainNode variant expression (excluding Text, which is inlined in collect_node).
fn build_plain_node_expr(node: &NodeDef) -> TokenStream {
    let type_str = node.node_type.to_string();

    let variant = Ident::new(&type_str.to_pascal_case(), Span::call_site());
    let plain_struct = format_ident!("Plain{}Node", variant);

    if node.params.is_empty() {
        quote! { PlainNode::#variant(#plain_struct::default()) }
    } else {
        let field_assigns: Vec<TokenStream> = node
            .params
            .iter()
            .map(|fv| {
                let name = &fv.name;
                let value = &fv.value;
                quote! { #name: #value }
            })
            .collect();
        quote! {
            PlainNode::#variant({
                #[allow(clippy::needless_update)]
                let __plain = #plain_struct {
                    #(#field_assigns,)*
                    ..Default::default()
                };
                __plain
            })
        }
    }
}

fn build_modifiers_expr(node: &NodeDef) -> TokenStream {
    let is_root = node.node_type == "root";

    if is_root {
        match &node.modifiers {
            None => quote! {
                {
                    let mut __mods: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
                    for __m in default_modifiers() {
                        __mods.insert(Modifier::as_type(&__m), __m);
                    }
                    __mods
                }
            },
            Some(mods) if mods.is_empty() => quote! {
                BTreeMap::new()
            },
            Some(mods) => {
                let modifier_exprs: Vec<TokenStream> =
                    mods.iter().map(build_modifier_expr).collect();
                quote! {
                    {
                        let mut __mods: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
                        for __m in default_modifiers_with(vec![#(#modifier_exprs),*]) {
                            __mods.insert(Modifier::as_type(&__m), __m);
                        }
                        __mods
                    }
                }
            }
        }
    } else {
        match &node.modifiers {
            None => quote! { BTreeMap::new() },
            Some(mods) if mods.is_empty() => quote! { BTreeMap::new() },
            Some(mods) => {
                let entries: Vec<TokenStream> = mods
                    .iter()
                    .map(|dec| {
                        let expr = build_modifier_expr(dec);
                        quote! {
                            {
                                let __m = #expr;
                                __mods.insert(Modifier::as_type(&__m), __m);
                            }
                        }
                    })
                    .collect();
                quote! {
                    {
                        let mut __mods: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
                        #(#entries)*
                        __mods
                    }
                }
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
            assert_eq!(
                exprs.len(),
                1,
                "positional modifier shorthand expects exactly one argument"
            );
            let expr = &exprs[0];
            quote! { Modifier::#variant { value: #expr } }
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
