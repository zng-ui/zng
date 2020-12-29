use proc_macro2::Ident;
use quote::*;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(item: proc_macro::TokenStream, trait_: Ident) -> proc_macro::TokenStream {
    let service = parse_macro_input!(item as DeriveInput);
    let value = ident!("{}Value", trait_);
    let entry = ident!("{}Entry", trait_);
    let static_ = ident!("TL_{}_ENTRY", trait_.to_string().to_uppercase());
    let ident = &service.ident;
    let crate_ = crate::util::crate_core();
    let r = quote! {
        impl #ident {
            std::thread_local! {
                static #static_: #crate_::service::#value<#ident> = #crate_::service::#value::init();
            }
        }

        impl #crate_::service::#trait_ for #ident {
            #[inline]
            fn thread_local_entry() -> #crate_::service::#entry<Self> {
                #crate_::service::#entry::new(&Self::#static_)
            }
        }
    };

    r.into()
}
