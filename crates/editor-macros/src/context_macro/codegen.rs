use proc_macro2::TokenStream;
use quote::quote;

use super::parse::ContextAst;

pub fn generate(ast: &ContextAst) -> TokenStream {
    match ast {
        ContextAst::Any => quote! { editor_model::ContextExpr::Any },
        ContextAst::SelfRef => quote! { editor_model::ContextExpr::SelfRef },
        ContextAst::GlobStar => quote! { editor_model::ContextExpr::GlobStar },
        ContextAst::Node(ident) => {
            quote! { editor_model::ContextExpr::Node(editor_model::NodeType::#ident) }
        }
        ContextAst::Child(parent, child) => {
            let p = generate(parent);
            let c = generate(child);
            quote! {
                editor_model::ContextExpr::Child {
                    parent: Box::new(#p),
                    child: Box::new(#c),
                }
            }
        }
        ContextAst::AnyOf(alts) => {
            let items: Vec<_> = alts.iter().map(generate).collect();
            quote! {
                editor_model::ContextExpr::AnyOf(vec![#(#items),*])
            }
        }
        ContextAst::Not(inner) => {
            let i = generate(inner);
            quote! { editor_model::ContextExpr::Not(Box::new(#i)) }
        }
    }
}
