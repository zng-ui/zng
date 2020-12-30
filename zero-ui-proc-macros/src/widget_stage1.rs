use crate::util;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::*, punctuated::Punctuated, *};

/// `widget!` entry.
///
/// ## In Stage 1:
///
/// 1 - Parse header, `$(#[$attr:meta])* $vis:vis $name:ident $(: $($inherited:path)++)? ;`.
/// 2 - Include `implicit_mixin` inheritance for widgets.
/// 3 - Generate warnings for header.
/// 4 - Generate a `mod` that uses the inherits to validate that they are imported modules.
/// 5 - Generate call to the first widget to include its internals.
///     This widget call is the start of **Stage 2**.
///
/// Because macros only expand from outermost first, we need to make the inherited
/// macros be called outermost, so we call then with all the input data from the new widget
/// they then include the inheritance data and routes further until all inherited data is included.
pub fn expand(mixin: bool, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // 1
    let mut wgt = parse_macro_input!(input as WidgetDeclaration);
    let crate_ = util::crate_core();
    if !mixin {
        // 2 - include `implicit_mixin` only for widgets.
        if wgt.header.inherit_start.is_none() {
            wgt.header.inherit_start = Some(parse_quote!(:));
        }
        wgt.header.inherits.push(parse_quote!(#crate_::widget_base::implicit_mixin));
    } else if wgt.header.inherits.is_empty() {
        // if we don't need to inherit anything, jumps to State 3.
        return super::widget_stage3::expand(quote! { mixin: true #wgt }.into());
    }

    let mut output = TokenStream::new();

    // 3 - TODO warning for repeated inherits, when the warnings API is sable.

    // 4
    let assert_inherits = wgt.header.inherits.iter();
    let wgt_init_asserts = ident!("{}_asserts", wgt.header.name);
    output.extend(quote! {
        #[allow(unused)]
        #[doc(hidden)]
        mod #wgt_init_asserts {
            use super::*;
            #(use #assert_inherits;)*
        }
    });

    // 5
    let mut inherits = wgt.header.inherits.clone();
    let first_inherit = inherits.pop().unwrap().into_value();

    let stage3_entry = ident!("{}_stg3_{}", wgt.header.name, util::uuid());
    output.extend(quote! {
        // the idea of this macro is that the `widget_stage3` in the declaration
        // span of the new widget.
        #[doc(hidden)]
        macro_rules! #stage3_entry {
            ($($tt:tt)*) => {
                #crate_::widget_stage3!{$($tt)*}
            }
        }

        // call for inherits[0]
        #first_inherit! {
            -> inherit {
                #stage3_entry;
                #first_inherit;
                // inherits[1..]
                #inherits
            }
            mixin: #mixin
            #wgt
        }
    });

    output.into()
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
        let attrs = Attribute::parse_outer(input)?;
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
