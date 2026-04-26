use proc_macro2::TokenStream;
use quote::quote;

use super::parse::ContentAst;

pub fn generate(ast: &ContentAst) -> TokenStream {
    match ast {
        ContentAst::Empty => quote! { editor_model::ContentExpr::Empty },
        ContentAst::Single(ident) => {
            quote! { editor_model::ContentExpr::Single(editor_model::NodeType::#ident) }
        }
        ContentAst::Seq(items) => {
            let items: Vec<_> = items.iter().map(generate).collect();
            quote! { editor_model::ContentExpr::Seq(vec![#(#items),*]) }
        }
        ContentAst::Choice(idents) => {
            let items: Vec<_> = idents
                .iter()
                .map(|i| quote! { editor_model::ContentExpr::Single(editor_model::NodeType::#i) })
                .collect();
            quote! { editor_model::ContentExpr::Choice(vec![#(#items),*]) }
        }
        ContentAst::ZeroOrMore(inner) => {
            let inner = generate(inner);
            quote! { editor_model::ContentExpr::ZeroOrMore(Box::new(#inner)) }
        }
        ContentAst::OneOrMore(inner) => {
            let inner = generate(inner);
            quote! { editor_model::ContentExpr::OneOrMore(Box::new(#inner)) }
        }
        ContentAst::Optional(inner) => {
            let inner = generate(inner);
            quote! { editor_model::ContentExpr::Optional(Box::new(#inner)) }
        }
    }
}
