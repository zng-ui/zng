extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use quote::__rt::Span;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Ident, ImplItem, ItemImpl};

macro_rules! error {
    ($span: expr, $msg: expr) => {{
        let error = quote_spanned! {
            $span=>
            compile_error!(concat!("#[impl_ui] ", $msg));
        };

        return TokenStream::from(error);
    }};
}

#[proc_macro_attribute]
pub fn impl_ui(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemImpl);

    if let Some((_, trait_, _)) = input.trait_ {
        error!(trait_.span(), "expected type impl found trait")
    }

    let ui_marker = ref_ident("ui");

    let mut ui_items = vec![];
    let mut other_items = vec![];

    for item in input.items {
        let is_ui = if let ImplItem::Method(m) = &mut item {
            if m.attrs.iter().index(|a| a.path.get_ident() == Some(&ui_marker)) {

            } else {
                false
            }
        } else {
            false
        };

        if is_ui {
            ui_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let impl_ui = ref_ident("impl_ui");
    let mut impl_attrs = input.attrs;
    impl_attrs.retain(|a| a.path.get_ident() != Some(&impl_ui));

    let unsafe_ = input.unsafety;

    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let result = quote! {
        #(#impl_attrs)*
        #unsafe_ impl #impl_generics #self_ty #ty_generics #where_clause {
            #(#other_items)*
        }
    };

    //impl Ui for Type {
    //        #(#ui_items)*
    //    }
    //
    TokenStream::from(result)
}

fn ref_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}
