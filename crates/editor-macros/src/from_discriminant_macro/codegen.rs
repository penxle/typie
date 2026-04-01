use proc_macro2::TokenStream;
use quote::quote;

use super::parse::FromDiscriminantInput;

pub fn generate(input: &FromDiscriminantInput) -> TokenStream {
    let enum_name = &input.enum_name;
    let discriminant_type = &input.discriminant_type;

    let match_arms: Vec<_> = input
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let inner_type = &v.inner_type;
            quote! {
                #discriminant_type::#variant_name => Self::#variant_name(#inner_type::default())
            }
        })
        .collect();

    quote! {
        impl #enum_name {
            pub fn from_discriminant(discriminant: #discriminant_type) -> Self {
                match discriminant {
                    #(#match_arms),*
                }
            }
        }
    }
}
