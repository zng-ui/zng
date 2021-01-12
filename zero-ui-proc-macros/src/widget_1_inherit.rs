use proc_macro2::TokenStream;
use syn::{parse::Parse, Ident, Path, Token};

use crate::util;

/// Takes the first path from the `inherit` section and turns it into an `__inherit!` call.
/// If the `inherit` section is empty calls `widget_declare!`.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<Input>(input).unwrap_or_else(|e| non_user_error!(e));
    let mixin = input.mixin;
    let rest = input.rest;
    let r = if let Some((inherit, _)) = input.inherit.next_path {
        let inherit_rest = input.inherit.rest;
        quote! {
            #inherit::__inherit! {
                mixin { #mixin }
                inherit { #inherit_rest }
                #rest
            }
        }
    } else {
        let crate_core = util::crate_core();
        quote! {
            #crate_core::widget_declare! {
                mixin { #mixin }
                inherit { }
                #rest
            }
        }
    };

    r.into()
}

struct Input {
    mixin: bool,
    inherit: Inherit,
    rest: TokenStream,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            // mixin { #LitBool }
            mixin: util::non_user_braced_id(input, "mixin").parse::<syn::LitBool>()?.value,
            // inherit { #( #Path ; )* }, only the first path is parsed.
            inherit: util::non_user_braced_id(input, "inherit").parse()?,
            // inherited and new widget data without parsing.
            rest: input.parse()?,
        })
    }
}

struct Inherit {
    /// First inherit path.
    next_path: Option<(Path, Token![;])>,
    /// Other inherit paths without parsing.
    rest: TokenStream,
}

impl Parse for Inherit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            // peeked a path segment.
            Ok(Inherit {
                next_path: Some((input.parse()?, input.parse()?)),
                rest: input.parse()?,
            })
        } else {
            // did not peeked a path segment, assert it is empty.
            let r = Inherit {
                next_path: None,
                rest: input.parse()?,
            };
            assert!(r.rest.is_empty());
            Ok(r)
        }
    }
}
