use crate::util;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::*, punctuated::Punctuated, *};
use uuid::Uuid;

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

    let stage3_entry = ident!("{}_stg3_{}", wgt.header.name, Uuid::new_v4().to_simple());

    // go to widget_stage2.
    let r = quote! {
        #[doc(hidden)]
        macro_rules! #stage3_entry {
            ($($tt:tt)*) => {
                widget_stage3!{$($tt)*}
            }
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

pub(crate) struct WidgetHeader {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    inherit_start: Option<Token![:]>,
    inherits: Punctuated<Path, Token![+]>,
    end: Token![;],
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
