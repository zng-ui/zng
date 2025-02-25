use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::{Ident, LitStr, ext::IdentExt, parse::Parse, parse_macro_input};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { unic_langid, lang } = parse_macro_input!(input as Input);

    let (raw, e_span) = match lang {
        LangInput::LitStr(s) => (s.value(), s.span()),
        LangInput::Ident(i) => (i.to_string(), i.span()),
    };

    let r = match unic_langid::LanguageIdentifier::from_str(&raw) {
        Ok(lang) => {
            let (lang, script, region, variants) = lang.into_parts();

            let lang: Option<u64> = lang.into();
            let lang = if let Some(lang) = lang {
                quote!(unsafe { #unic_langid::subtags::Language::from_raw_unchecked(#lang) })
            } else {
                quote!(std::default::Default::default())
            };

            let script = if let Some(script) = script {
                let script: u32 = script.into();
                quote!(Some(unsafe { #unic_langid::subtags::Script::from_raw_unchecked(#script) }))
            } else {
                quote!(None)
            };

            let region = if let Some(region) = region {
                let region: u32 = region.into();
                quote!(Some(unsafe { #unic_langid::subtags::Region::from_raw_unchecked(#region) }))
            } else {
                quote!(None)
            };

            let variants = if !variants.is_empty() {
                let v: Vec<_> = variants
                    .iter()
                    .map(|v| {
                        let variant: u64 = v.into();
                        quote!(unsafe { #unic_langid::subtags::Variant::from_raw_unchecked(#variant) })
                    })
                    .collect();
                quote!(Some(Box::new([#(#v,)*])))
            } else {
                quote!(None)
            };

            quote! {
                #unic_langid::LanguageIdentifier::from_raw_parts_unchecked(#lang, #script, #region, #variants)
            }
        }
        Err(e) => {
            let e = match e {
                unic_langid::LanguageIdentifierError::Unknown => "unknown error".to_owned(),
                unic_langid::LanguageIdentifierError::ParserError(e) => e.to_string(),
            };
            quote_spanned! {e_span=>
                compile_error!(#e)
            }
        }
    };

    r.into()
}

struct Input {
    unic_langid: TokenStream,
    lang: LangInput,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            unic_langid: non_user_braced!(input, "unic_langid").parse().unwrap(),
            lang: non_user_braced!(input, "lang").parse().unwrap(),
        })
    }
}

enum LangInput {
    LitStr(LitStr),
    Ident(Ident),
}
impl Parse for LangInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident::peek_any) {
            Ok(LangInput::Ident(input.parse()?))
        } else {
            Ok(LangInput::LitStr(input.parse()?))
        }
    }
}
