use heck::ToPascalCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::doc_macro::parse::{
    DecorationDef, DecorationParams, DocTree, FieldValue, MarkerDef, NodeContent, NodeDef,
};

pub struct CodegenParts {
    pub root_entry: TokenStream,
    pub bindings: Vec<(Ident, Vec<usize>)>,
}

pub fn generate_parts(tree: &DocTree) -> CodegenParts {
    let mut bindings = Vec::new();

    let root_entry = collect_node(&tree.root, &mut Vec::new(), &mut bindings);

    CodegenParts {
        root_entry,
        bindings,
    }
}

// Returns the `PlainNodeEntry { .. }` construction TokenStream for this node,
// building its children inline. Records each user-labeled node's binding ident
// together with its child-index path from the root.
fn collect_node(
    node: &NodeDef,
    path: &mut Vec<usize>,
    bindings: &mut Vec<(Ident, Vec<usize>)>,
) -> TokenStream {
    let is_text = node.node_type == "text";

    if let Some(ref binding) = node.binding {
        bindings.push((binding.clone(), path.clone()));
    }

    let child_entries: Vec<TokenStream> = match &node.content {
        NodeContent::Children(children) => children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                path.push(i);
                let entry = collect_node(child, path, bindings);
                path.pop();
                entry
            })
            .collect(),
        NodeContent::Text(_) | NodeContent::Leaf => vec![],
    };

    let children_expr = if child_entries.is_empty() {
        quote! { vec![] }
    } else {
        quote! { vec![#(#child_entries),*] }
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

    let marker_expr = build_marker_expr(&node.marker);

    quote! {
        PlainNodeEntry {
            node: #plain_node_with_text,
            modifiers: #modifiers_expr,
            marker: #marker_expr,
            children: #children_expr,
        }
    }
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

fn build_marker_expr(marker: &Option<MarkerDef>) -> TokenStream {
    match marker {
        Some(m) => {
            let mod_exprs: Vec<TokenStream> = m.modifiers.iter().map(build_modifier_expr).collect();
            quote! { Some(Marker { modifiers: vec![#(#mod_exprs),*] }) }
        }
        None => quote! { None },
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
