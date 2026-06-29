use heck::ToPascalCase;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use super::parse::{AffinityKind, PendingModifierDef, PositionExpr, SelectionExpr, StateInput};
use crate::doc_macro;
use crate::doc_macro::codegen::build_modifier_expr;

pub fn generate(input: &StateInput) -> TokenStream {
    let parts = doc_macro::codegen::generate_parts(&input.doc_tree);
    let root_entry = &parts.root_entry;
    let style_entries = &parts.style_entries;

    let binding_idents: Vec<&Ident> = parts.bindings.iter().map(|(ident, _)| ident).collect();
    let binding_resolves: Vec<TokenStream> = parts
        .bindings
        .iter()
        .map(|(ident, path)| {
            let path_lits: Vec<Literal> =
                path.iter().map(|i| Literal::usize_suffixed(*i)).collect();
            quote! {
                let #ident = __handles
                    .get::<[usize]>(&[#(#path_lits),*])
                    .cloned()
                    .expect("state! binding resolves to a projected block");
            }
        })
        .collect();

    let selection_expr = gen_selection(&input.selection);
    let pending_modifiers_expr = gen_pending_modifiers(&input.pending_modifiers);

    quote! {
        {
            use ::editor_model::*;
            use ::std::collections::BTreeMap;

            let __plain = PlainDoc {
                root: #root_entry,
                styles: {
                    let mut __s: BTreeMap<String, PlainStyleEntry> = BTreeMap::new();
                    #(#style_entries)*
                    __s
                },
            };

            let (mut state, __handles) =
                ::editor_state::test_utils::build_state_from_plain(__plain);

            #(#binding_resolves)*

            state.selection = #selection_expr;
            #pending_modifiers_expr

            (state, #(#binding_idents),*)
        }
    }
}

fn gen_selection(sel: &SelectionExpr) -> TokenStream {
    match sel {
        SelectionExpr::None => quote! { None },
        SelectionExpr::Collapsed(pos) => {
            let pos_expr = gen_position(pos);
            quote! { Some(::editor_state::Selection::collapsed(#pos_expr)) }
        }
        SelectionExpr::Range(anchor, head) => {
            let anchor_expr = gen_position(anchor);
            let head_expr = gen_position(head);
            quote! {
                Some(::editor_state::Selection::new(#anchor_expr, #head_expr))
            }
        }
    }
}

fn gen_position(pos: &PositionExpr) -> TokenStream {
    let node = &pos.node_ident;
    let offset = &pos.offset;

    let node_expr = quote! { #node.clone() };
    let offset_expr = quote! { #offset };

    match &pos.affinity {
        None => {
            quote! {
                ::editor_state::Position::new(#node_expr, #offset_expr)
            }
        }
        Some(kind) => {
            let affinity = match kind {
                AffinityKind::Upstream => quote! { ::editor_state::Affinity::Upstream },
                AffinityKind::Downstream => quote! { ::editor_state::Affinity::Downstream },
            };
            quote! {
                ::editor_state::Position {
                    node: #node_expr,
                    offset: #offset_expr,
                    affinity: #affinity,
                }
            }
        }
    }
}

fn gen_pending_modifiers(modifiers: &[PendingModifierDef]) -> TokenStream {
    if modifiers.is_empty() {
        return quote! {};
    }

    let push_exprs: Vec<TokenStream> = modifiers
        .iter()
        .map(|def| match def {
            PendingModifierDef::Set(dec) => {
                let modifier_expr = build_modifier_expr(dec);
                quote! {
                    __pending.push(::editor_state::PendingModifier::Set { modifier: #modifier_expr });
                }
            }
            PendingModifierDef::Unset(name) => {
                let variant = Ident::new(&name.to_string().to_pascal_case(), Span::call_site());
                quote! {
                    __pending.push(::editor_state::PendingModifier::Unset { ty: ModifierType::#variant });
                }
            }
        })
        .collect();

    quote! {
        {
            let mut __pending = ::editor_state::PendingModifiers::new();
            #(#push_exprs)*
            state.pending_modifiers = __pending;
        }
    }
}
