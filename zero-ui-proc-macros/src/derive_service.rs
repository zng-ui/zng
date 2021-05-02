use quote::*;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let service = parse_macro_input!(item as DeriveInput);
    let ident = &service.ident;
    let crate_ = crate::util::crate_core();
    let r = quote! {
        impl #ident {
            std::thread_local! {
                static TL_SERVICE_ENTRY: #crate_::service::ServiceValue<#ident> = #crate_::service::ServiceValue::init();
            }
        }

        impl #crate_::service::Service for #ident {
            #[inline]
            fn thread_local_entry() -> #crate_::service::ServiceEntry<Self> {
                #crate_::service::ServiceEntry::new(&Self::TL_SERVICE_ENTRY)
            }
        }
    };

    r.into()
}
