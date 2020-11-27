use quote::*;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let service = parse_macro_input!(item as DeriveInput);
    let ident = &service.ident;
    let crate_ = crate::util::zero_ui_crate_ident();
    let r = quote! {
        impl #ident {
            std::thread_local! {
                static THREAD_LOCAL_ENTRY: #crate_::core::service::AppServiceValue<#ident> = #crate_::core::service::AppServiceValue::init();
            }
        }

        impl #crate_::core::service::AppService for #ident {
            #[inline]
            fn thread_local_entry() -> #crate_::core::service::AppServiceEntry<Self> {
                #crate_::core::service::AppServiceEntry::new(&Self::THREAD_LOCAL_ENTRY)
            }
        }
    };

    r.into()
}
