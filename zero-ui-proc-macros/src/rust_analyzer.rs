//! Macros that generate code for rust-analyzer only in cases where it does not work for the real macro and we can't change
//! the real macro to work with rust-analyzer

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, Attribute};

use crate::widget_new::{PropertyAssign, PropertyValue, UserInput, WhenExprToVar};

/// Fakes the `__widget_macro! { call_site { . } $($tt)* }` to make property expressions visible in rust-analyzer.
pub fn widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match syn::parse(input.clone()) {
        Ok(input) => clean_output(input).into(),
        Err(_) => input,
    }
}

struct Input {
    new: TokenStream,
    input: UserInput,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            new: non_user_braced!(input, "new").parse().unwrap(),
            input: input.parse()?,
        })
    }
}

fn clean_output(Input { new, input }: Input) -> TokenStream {
    let mut out = TokenStream::new();

    // property assign expressions
    for p in &input.properties {
        with_attrs(&mut out, &p.attrs, |out| {
            property_assign(out, p);
        });
    }
    for when in &input.whens {
        let mut inner = TokenStream::new();
        for p in &when.assigns {
            with_attrs(&mut inner, &p.attrs, |out| {
                property_assign(out, p);
            });
        }

        if let Ok(c) = syn::parse2::<WhenExprToVar>(when.condition_expr.clone()) {
            let c = &c.expr;
            inner.extend(quote! {
                drop({
                    #c
                });
            });
        }

        for attr in &when.attrs {
            attr.to_tokens(&mut out);
        }
        out.extend(quote! {
            {
                #inner
            }
        })
    }

    quote! {
        {
            #out
            #new
        }
    }
}

fn with_attrs(out: &mut TokenStream, attrs: &[Attribute], action: impl FnOnce(&mut TokenStream)) {
    if attrs.is_empty() {
        action(out);
    } else {
        for attr in attrs {
            attr.to_tokens(out);
        }
        let mut inner = TokenStream::new();
        action(&mut inner);
        out.extend(quote! {
            {
                #inner
            }
        })
    }
}

fn property_assign(out: &mut TokenStream, p: &PropertyAssign) {
    match &p.value {
        PropertyValue::Special(_, _) => {}
        PropertyValue::Unnamed(args) => {
            for arg in args {
                out.extend(quote! {
                    drop(#arg);
                });
            }
        }
        PropertyValue::Named(_, fields) => {
            for field in fields {
                let value = &field.expr;
                out.extend(quote! {
                    drop(#value);
                });
            }
        }
    }
}
