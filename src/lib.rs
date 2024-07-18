use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(CopyNonDefaults)]
pub fn copy_non_defaults_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let gen = match input.data {
        Data::Struct(ref data) => {
            let fields = match &data.fields {
                Fields::Named(ref fields_named) => &fields_named.named,
                Fields::Unnamed(_) => panic!("Unnamed fields are not supported"),
                Fields::Unit => panic!("Unit structs are not supported"),
            };

            let mut field_copies = Vec::with_capacity(fields.len());

            for field in fields {
                let field_name = &field.ident;

                field_copies.push(quote! {
                    if other.#field_name != def.#field_name {
                        self.#field_name = other.#field_name.clone();
                    }
                });
            }

            quote! {
                impl #name {
                    pub fn copy_non_defaults(&mut self, other: &#name) {
                        let def = Self::default();
                        #(#field_copies)*
                    }
                }
            }
        }
        _ => panic!("CopyNonDefaults can only be derived for structs"),
    };
    gen.into()
}
