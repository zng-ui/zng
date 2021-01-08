#![allow(unused)] // TODO remove after expand is called in lib.rs.

use proc_macro2::TokenStream;
use syn::{parse::Parse, Ident, Path, Token};

use crate::util;

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<Input>(input).unwrap_or_else(|e| non_user_error!(e));
    let rest = input.rest;
    let r = if let Some((inherit, _)) = input.inherit.next_path {
        let inherit_rest = input.inherit.rest;
        quote! {
            #inherit::__inherit! {
                inherit { #inherit_rest }
                #rest
            }
        }
    } else {
        let crate_core = util::crate_core();
        quote! {
            #crate_core::widget_declare! {
                #rest
            }
        }
    };

    r.into()
}

struct Input {
    inherit: Inherit,
    rest: TokenStream,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            inherit: util::non_user_braced_id(input, "inherit").parse()?,
            rest: input.parse()?,
        })
    }
}

struct Inherit {
    next_path: Option<(Path, Token![;])>,
    rest: TokenStream,
}

impl Parse for Inherit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            Ok(Inherit {
                next_path: Some((input.parse()?, input.parse()?)),
                rest: input.parse()?,
            })
        } else {
            let r = Inherit {
                next_path: None,
                rest: input.parse()?,
            };
            assert!(r.rest.is_empty());
            Ok(r)
        }
    }
}
