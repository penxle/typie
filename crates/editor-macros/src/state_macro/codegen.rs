use heck::ToPascalCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use super::parse::{AffinityKind, PendingModifierDef, PositionExpr, SelectionExpr, StateInput};
use crate::doc_macro;
use crate::doc_macro::codegen::build_modifier_expr;

pub fn generate(input: &StateInput) -> TokenStream {
    let parts = doc_macro::codegen::generate_parts(&input.doc_tree);
    let id_decls = &parts.id_decls;
    let with_nodes = &parts.with_nodes;
    let bindings = &parts.bindings;

    let selection_expr = gen_selection(&input.selection);
    let pending_modifiers_expr = gen_pending_modifiers(&input.pending_modifiers);

    quote! {
        {
            use editor_model::*;
            use editor_state::*;

            #(#id_decls)*

            let doc = Doc::new_test();
            #(#with_nodes)*

            let selection = #selection_expr;
            let mut state = State::new(doc, selection);
            #pending_modifiers_expr

            (state, #(#bindings),*)
        }
    }
}

fn gen_selection(sel: &SelectionExpr) -> TokenStream {
    match sel {
        SelectionExpr::Collapsed(pos) => {
            let pos_expr = gen_position(pos);
            quote! { Selection::collapsed(#pos_expr) }
        }
        SelectionExpr::Range(anchor, head) => {
            let anchor_expr = gen_position(anchor);
            let head_expr = gen_position(head);
            quote! { Selection::new(#anchor_expr, #head_expr) }
        }
    }
}

fn gen_position(pos: &PositionExpr) -> TokenStream {
    let node = &pos.node_ident;
    let offset = &pos.offset;

    match &pos.affinity {
        None => {
            quote! { Position::new(#node, #offset) }
        }
        Some(kind) => {
            let affinity = match kind {
                AffinityKind::Upstream => quote! { Affinity::Upstream },
                AffinityKind::Downstream => quote! { Affinity::Downstream },
            };
            quote! {
                Position { node_id: #node, offset: #offset, affinity: #affinity }
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
                quote! { __pending.push(PendingModifier::Set(#modifier_expr)); }
            }
            PendingModifierDef::Unset(name) => {
                let variant = Ident::new(&name.to_string().to_pascal_case(), Span::call_site());
                quote! { __pending.push(PendingModifier::Unset(ModifierType::#variant)); }
            }
        })
        .collect();

    quote! {
        {
            let mut __pending = PendingModifiers::new();
            #(#push_exprs)*
            state.pending_modifiers = __pending;
        }
    }
}
