use quote::*;
use syn::{parse_macro_input, DeriveInput};

use crate::util::snake_case;

pub fn derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let service = parse_macro_input!(item as DeriveInput);
    let ident = &service.ident;
    let crate_ = crate::util::crate_core();

    let ext_ident = ident!("{}Ext", ident);
    let ext_mtd_ident = ident!("{}", snake_case(&ident.to_string()));
    let ext_help = format!(
        "Adds the [`{}`] method to [`Services`]({}::service::Services)",
        ext_mtd_ident,
        crate_.to_string().replace(" ", "")
    );
    let ext_mtd_help = format!(
        "Requires the [`{0}`] service. This is the equivalent of calling `services.req::<{0}>()`",
        ident
    );

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

        #[doc=#ext_help]
        pub trait #ext_ident {
            #[doc=#ext_mtd_help]
            fn #ext_mtd_ident(&mut self)  -> &mut #ident;
        }
        impl #ext_ident for #crate_::service::Services {
            #[inline]
            #[track_caller]
            fn #ext_mtd_ident(&mut self) -> &mut #ident {
                self.req::<#ident>()
            }
        }
    };

    r.into()
}
