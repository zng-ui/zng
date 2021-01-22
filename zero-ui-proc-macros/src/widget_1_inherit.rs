use proc_macro2::TokenStream;
use syn::{parse::Parse, Ident, Path, Token};

use crate::util;

/// Takes the first path from the `inherit` section and turns it into an `__inherit!` call.
/// If the `inherit` section is empty calls `widget_declare!`.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<Input>(input).unwrap_or_else(|e| non_user_error!(e));
    let rest = input.rest;
    let r = if let Some((inherit, _)) = input.inherit.next_path {
        let inherit_rest = input.inherit.rest;
        quote! {
            // TODO support #[cfg(..)] in inherit!(..).
            #inherit::__inherit! {
                inherit { #inherit_rest }
                #rest
            }
        }
    } else {
        let crate_core = util::crate_core();
        quote! {
            #crate_core::widget_declare! {
                inherit { }
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
            // inherit { #( #Path ; )* }, only the first path is parsed.
            inherit: non_user_braced!(input, "inherit").parse()?,
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
