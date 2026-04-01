use proc_macro2::TokenStream;
use quote::quote;

use super::parse::ContextAst;

pub fn generate(ast: &ContextAst) -> TokenStream {
    match ast {
        ContextAst::Any => quote! { editor_schema::ContextExpr::Any },
        ContextAst::SelfRef => quote! { editor_schema::ContextExpr::SelfRef },
        ContextAst::GlobStar => quote! { editor_schema::ContextExpr::GlobStar },
        ContextAst::Node(ident) => {
            quote! { editor_schema::ContextExpr::Node(editor_model::NodeType::#ident) }
        }
        ContextAst::Child(parent, child) => {
            let p = generate(parent);
            let c = generate(child);
            quote! {
                editor_schema::ContextExpr::Child {
                    parent: Box::new(#p),
                    child: Box::new(#c),
                }
            }
        }
        ContextAst::AnyOf(alts) => {
            let items: Vec<_> = alts.iter().map(generate).collect();
            quote! {
                editor_schema::ContextExpr::AnyOf(vec![#(#items),*])
            }
        }
        ContextAst::Not(inner) => {
            let i = generate(inner);
            quote! { editor_schema::ContextExpr::Not(Box::new(#i)) }
        }
    }
}
