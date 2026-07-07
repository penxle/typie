use heck::ToPascalCase;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use super::parse::{AffinityKind, PendingModifierDef, PositionExpr, SelectionExpr, StateInput};
use crate::doc_macro;
use crate::doc_macro::codegen::build_modifier_expr;

pub fn generate(input: &StateInput) -> TokenStream {
    let parts = doc_macro::codegen::generate_parts(&input.doc_tree);
    let root_entry = &parts.root_entry;

    let binding_idents: Vec<&Ident> = parts
        .bindings
        .iter()
        .map(|binding| &binding.ident)
        .collect();
    let synthetic_checks: Vec<TokenStream> = parts
        .synthetic_checks
        .iter()
        .map(|check| {
            let node_expr = gen_projected_element_id(&check.path, Some(&check.node_type), true);
            quote! {
                let _ = #node_expr;
            }
        })
        .collect();
    let binding_resolves: Vec<TokenStream> = parts
        .bindings
        .iter()
        .map(|binding| {
            let ident = &binding.ident;
            if binding.projected {
                let node_expr = gen_projected_element_id(&binding.path, None, false);
                quote! {
                    let #ident = #node_expr;
                }
            } else {
                let path_lits: Vec<Literal> = binding
                    .path
                    .iter()
                    .map(|i| Literal::usize_suffixed(*i))
                    .collect();
                quote! {
                    let #ident = __handles
                        .get::<[usize]>(&[#(#path_lits),*])
                        .cloned()
                        .expect("state! binding resolves to a projected block");
                }
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
            };

            let (mut state, __handles) =
                ::editor_state::test_utils::build_state_from_plain(__plain);

            #(#synthetic_checks)*
            #(#binding_resolves)*

            state.selection = #selection_expr;
            #pending_modifiers_expr

            (state, #(#binding_idents),*)
        }
    }
}

fn gen_projected_element_id(
    path: &[usize],
    expected_type: Option<&Ident>,
    require_synthetic: bool,
) -> TokenStream {
    let (parent_path, last_index) = match path.split_last() {
        Some((last, parent)) => (parent, Some(Literal::usize_suffixed(*last))),
        None => (&[][..], None),
    };
    let parent_path_lits: Vec<Literal> = parent_path
        .iter()
        .map(|i| Literal::usize_suffixed(*i))
        .collect();

    let type_check = expected_type.map(|node_type| {
        let variant = Ident::new(&node_type.to_string().to_pascal_case(), Span::call_site());
        quote! {
            assert_eq!(
                __node_type,
                ::editor_model::NodeType::#variant,
                "state! synthetic placeholder resolved to an unexpected node type",
            );
        }
    });

    let synthetic_check = if require_synthetic {
        quote! {
            assert!(
                __id.is_synthetic(),
                "state! synthetic placeholder resolved to a real node",
            );
        }
    } else {
        quote! {}
    };

    let element_expr = match last_index {
        Some(last_index) => quote! {
            match __node
                .child_at(#last_index)
                .expect("state! synthetic placeholder path exists in projection")
            {
                ::editor_model::ChildView::Block(__block) => (__block.id(), __block.node_type()),
                ::editor_model::ChildView::Leaf(__leaf) => (__leaf.dot(), __leaf.node_type()),
            }
        },
        None => quote! {
            (__node.id(), __node.node_type())
        },
    };

    quote! {
        {
            let __view = state.view();
            let mut __node = __view.root().expect("state! projected document has a root");
            #(
                __node = match __node
                    .child_at(#parent_path_lits)
                    .expect("state! synthetic placeholder path exists in projection")
                {
                    ::editor_model::ChildView::Block(__block) => __block,
                    ::editor_model::ChildView::Leaf(_) => {
                        panic!("state! synthetic placeholder path crosses a leaf")
                    }
                };
            )*
            let (__id, __node_type) = #element_expr;
            #type_check
            #synthetic_check
            __id
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
