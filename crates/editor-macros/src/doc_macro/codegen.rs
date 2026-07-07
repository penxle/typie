use heck::ToPascalCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::doc_macro::parse::{
    CarryDef, DecorationDef, DecorationParams, DocTree, FieldValue, NodeContent, NodeDef,
};

pub struct CodegenParts {
    pub root_entry: TokenStream,
    pub bindings: Vec<BindingDef>,
    pub synthetic_checks: Vec<SyntheticCheck>,
}

pub struct BindingDef {
    pub ident: Ident,
    pub path: Vec<usize>,
    pub projected: bool,
}

pub struct SyntheticCheck {
    pub path: Vec<usize>,
    pub node_type: Ident,
}

pub fn generate_parts(tree: &DocTree) -> CodegenParts {
    let mut bindings = Vec::new();
    let mut synthetic_checks = Vec::new();

    let root_entry = collect_node(
        &tree.root,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut bindings,
        &mut synthetic_checks,
    )
    .expect("root is never synthetic");

    CodegenParts {
        root_entry,
        bindings,
        synthetic_checks,
    }
}

// Returns the `PlainNodeEntry { .. }` construction TokenStream for this node,
// building its children inline. Records each user-labeled node's binding ident
// together with its child-index path from the root.
fn collect_node(
    node: &NodeDef,
    plain_path: &mut Vec<usize>,
    projected_path: &mut Vec<usize>,
    bindings: &mut Vec<BindingDef>,
    synthetic_checks: &mut Vec<SyntheticCheck>,
) -> Option<TokenStream> {
    if node.synthetic {
        collect_synthetic_node(node, projected_path, bindings, synthetic_checks);
        return None;
    }

    let is_text = node.node_type == "text";

    if let Some(ref binding) = node.binding {
        bindings.push(BindingDef {
            ident: binding.clone(),
            path: plain_path.clone(),
            projected: false,
        });
    }

    let child_entries: Vec<TokenStream> = match &node.content {
        NodeContent::Children(children) => {
            let mut entries = Vec::new();
            let mut plain_index = 0usize;
            for (projected_index, child) in children.iter().enumerate() {
                projected_path.push(projected_index);
                if child.synthetic {
                    collect_synthetic_node(child, projected_path, bindings, synthetic_checks);
                } else {
                    plain_path.push(plain_index);
                    let entry = collect_node(
                        child,
                        plain_path,
                        projected_path,
                        bindings,
                        synthetic_checks,
                    )
                    .expect("non-synthetic child emits a PlainNodeEntry");
                    plain_path.pop();
                    entries.push(entry);
                    plain_index += 1;
                }
                projected_path.pop();
            }
            entries
        }
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

    let carry_expr = build_carry_expr(&node.carry);

    Some(quote! {
        PlainNodeEntry {
            node: #plain_node_with_text,
            modifiers: #modifiers_expr,
            carry: #carry_expr,
            children: #children_expr,
        }
    })
}

fn collect_synthetic_node(
    node: &NodeDef,
    projected_path: &mut Vec<usize>,
    bindings: &mut Vec<BindingDef>,
    synthetic_checks: &mut Vec<SyntheticCheck>,
) {
    if let Some(ref binding) = node.binding {
        bindings.push(BindingDef {
            ident: binding.clone(),
            path: projected_path.clone(),
            projected: true,
        });
    }
    synthetic_checks.push(SyntheticCheck {
        path: projected_path.clone(),
        node_type: node.node_type.clone(),
    });

    if let NodeContent::Children(children) = &node.content {
        for (index, child) in children.iter().enumerate() {
            projected_path.push(index);
            collect_synthetic_node(child, projected_path, bindings, synthetic_checks);
            projected_path.pop();
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

fn build_carry_expr(carry: &Option<CarryDef>) -> TokenStream {
    match carry {
        Some(c) => {
            let mod_exprs: Vec<TokenStream> = c.modifiers.iter().map(build_modifier_expr).collect();
            quote! { vec![#(#mod_exprs),*] }
        }
        None => quote! { Vec::new() },
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
