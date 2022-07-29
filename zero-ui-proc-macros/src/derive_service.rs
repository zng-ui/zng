use quote::*;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let service = parse_macro_input!(item as DeriveInput);
    let ident = &service.ident;
    let crate_ = crate::util::crate_core();

    let req_help = format!(
        "Requires the [`{0}`] service. This is the equivalent of calling `services.req::<{0}>()`",
        ident
    );
    let get_help = format!(
        "Tries to find the [`{0}`] service. This is the equivalent of calling `services.get::<{0}>()`",
        ident
    );

    let r = quote! {
        impl #ident {
            std::thread_local! {
                static TL_SERVICE_ENTRY: #crate_::service::ServiceValue<#ident> = #crate_::service::ServiceValue::init();
            }

            #[doc=#req_help]
            #[allow(unused)]
            pub fn req(services: &mut impl AsMut<#crate_::service::Services>) -> &mut Self {
                services.as_mut().req::<Self>()
            }

            #[doc=#get_help]
            #[allow(unused)]
            pub fn get(services: &mut impl AsMut<#crate_::service::Services>) -> Option<&mut Self> {
                services.as_mut().get::<Self>()
            }
        }

        impl #crate_::service::Service for #ident {

            fn thread_local_entry() -> #crate_::service::ServiceEntry<Self> {
                #crate_::service::ServiceEntry::new(&Self::TL_SERVICE_ENTRY)
            }
        }
    };

    r.into()
}
