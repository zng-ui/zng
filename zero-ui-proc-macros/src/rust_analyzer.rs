//! Macros that generate code for rust-analyzer only in cases where it does not work for the real macro and we can't change
//! the real macro to work with rust-analyzer

use proc_macro2::TokenStream;

use crate::widget_new::{UserInput, PropertyValue};

/// Fakes the `__widget_macro! { call_site { . } $($tt)* }` to make property expressions visible in rust-analyzer.
pub fn widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match syn::parse(input.clone()) {
        Ok(input) => {
            clean_output(input).into()
        },
        Err(_) => {
            input
        },
    }
}

fn clean_output(input: UserInput) -> TokenStream {
    let mut values = TokenStream::new();
    for p in input.properties {
        match p.value {
            PropertyValue::Special(_, _) => {},
            PropertyValue::Unnamed(args) => for arg in args {
                values.extend(quote! {
                    drop(#arg);
                });
            },
            PropertyValue::Named(_, fields) => for field in fields {
                let value = field.expr;
                values.extend(quote! {
                    drop(#value);
                });
            },
        }
    }

    quote! {
        #values
    }
}