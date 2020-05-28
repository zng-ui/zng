use crate::util;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::*, punctuated::Punctuated, *};

/// `widget!` entry, parse header and expands to calls to inherited widget macros to include
/// their internals.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut wgt = parse_macro_input!(input as WidgetDeclaration);
    if wgt.header.inherit_start.is_none() {
        wgt.header.inherit_start = Some(parse_quote!(:));
    }
    let crate_ = util::zero_ui_crate_ident();
    wgt.header.inherits.push(parse_quote!(#crate_::widgets::implicit_mixin));

    let mut inherits = wgt.header.inherits.clone();
    let first_inherit = inherits.pop().unwrap();

    let stage3_entry = ident!("{}_stg3_{}", wgt.header.name, util::uuid());

    let assert_inherits = wgt.header.inherits.iter();

    let wgt_init_asserts = ident!("__{}_asserts", wgt.header.name);

    // go to widget_stage2.
    let r = quote! {
        #[doc(hidden)]
        macro_rules! #stage3_entry {
            ($($tt:tt)*) => {
                widget_stage3!{$($tt)*}
            }
        }

        #[allow(unused)]
        #[doc(hidden)]
        mod #wgt_init_asserts {
            use super::*;
            #(use #assert_inherits;)*
        }

        #first_inherit! {
            -> inherit {
                #stage3_entry;
                #first_inherit;
                #inherits
            }
            #wgt
        }
    };

    r.into()
}

struct WidgetDeclaration {
    header: WidgetHeader,
    rest: TokenStream,
}

impl Parse for WidgetDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let header = input.parse()?;
        let rest = input.parse()?;
        Ok(WidgetDeclaration { header, rest })
    }
}

impl ToTokens for WidgetDeclaration {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.header.to_tokens(tokens);
        self.rest.to_tokens(tokens);
    }
}

pub struct WidgetHeader {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub inherit_start: Option<Token![:]>,
    pub inherits: Punctuated<Path, Token![+]>,
    pub end: Token![;],
}

impl Parse for WidgetHeader {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_inner(input)?;
        let vis = input.parse()?;
        let name = input.parse()?;
        let inherit_start: Option<Token![:]> = input.parse()?;
        let inherits = if inherit_start.is_some() {
            Punctuated::parse_separated_nonempty(input)?
        } else {
            Punctuated::new()
        };
        let end = input.parse()?;

        Ok(WidgetHeader {
            attrs,
            vis,
            name,
            inherit_start,
            inherits,
            end,
        })
    }
}

impl ToTokens for WidgetHeader {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }
        self.vis.to_tokens(tokens);
        self.name.to_tokens(tokens);
        self.inherit_start.to_tokens(tokens);
        self.inherits.to_tokens(tokens);
        self.end.to_tokens(tokens);
    }
}
