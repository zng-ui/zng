use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, Ident, Path, Token};

use crate::util;

/// Takes the first path from the `inherit` section and turns it into an `__inherit!` call.
/// If the `inherit` section is empty calls `widget_declare!`.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<Input>(input).unwrap_or_else(|e| non_user_error!(e));
    let rest = input.rest;
    let r = if let Some(inherit) = input.inherit.next_inherit {
        let inherit_rest = input.inherit.rest;
        let path = inherit.path;
        let cfg = inherit.cfg;
        let not_cfg = util::negate_cfg_attr(cfg.clone());
        quote! {
            #path::__inherit! {
                cfg { #cfg }
                not_cfg { #not_cfg }
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
            // inherit { #( #Path ; )* }, only the first path is parsed.
            inherit: non_user_braced!(input, "inherit").parse()?,
            // inherited and new widget data without parsing.
            rest: input.parse()?,
        })
    }
}

struct Inherit {
    /// First inherit path.
    next_inherit: Option<InheritItem>,
    /// Other inherit paths without parsing.
    rest: TokenStream,
}

impl Parse for Inherit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![#]) || input.peek(Ident) {
            // peeked an inherit item.
            Ok(Inherit {
                next_inherit: Some(input.parse()?),
                rest: input.parse()?,
            })
        } else {
            // did not peeked an inherit item, assert it is empty.
            let r = Inherit {
                next_inherit: None,
                rest: input.parse()?,
            };
            assert!(r.rest.is_empty());
            Ok(r)
        }
    }
}

struct InheritItem {
    cfg: Option<syn::Attribute>,
    path: Path,
    semi: Token![;],
}
impl Parse for InheritItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attrs = syn::Attribute::parse_outer(input)?;
        let cfg = attrs.pop();
        if !attrs.is_empty() {
            non_user_error!("expected none or single #[cfg(..)] attribute")
        }
        Ok(InheritItem {
            cfg,
            path: input.parse()?,
            semi: input.parse()?,
        })
    }
}
