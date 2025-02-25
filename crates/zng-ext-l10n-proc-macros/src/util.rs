use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};

/// Collection of compile errors.
#[derive(Default)]
pub struct Errors {
    tokens: TokenStream,
}
impl Errors {
    /// Push a compile error.
    pub fn push(&mut self, error: impl ToString, span: Span) {
        let error = error.to_string();
        self.tokens.extend(quote_spanned! {span=>
            compile_error!{#error}
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
impl ToTokens for Errors {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.tokens.clone())
    }
    fn to_token_stream(&self) -> TokenStream {
        self.tokens.clone()
    }
    fn into_token_stream(self) -> TokenStream {
        self.tokens
    }
}

/// Input error not caused by the user.
macro_rules! non_user_error {
    ($e:expr) => {
        panic!("[{}:{}] invalid non-user input: {}", file!(), line!(), $e)
    };
    ($fmt:tt, $($args:tt)+) => {
        non_user_error! {
            format_args!($fmt, $($args)+)
        }
    }
}

/// `Ident` with custom span.
macro_rules! ident_spanned {
    ($span:expr=> $($format_name:tt)+) => {
        proc_macro2::Ident::new(&format!($($format_name)+), $span)
    };
}

macro_rules! non_user_group {
    ($group_kind:ident, $input:expr) => {
        {
            fn inner(input: syn::parse::ParseStream) -> syn::Result<syn::parse::ParseBuffer> {
                let inner;
                // this macro inserts a return Err(..) but we want to panic
                syn::$group_kind!(inner in input);
                Ok(inner)
            }
            inner($input).unwrap_or_else(|e| non_user_error!(e))
        }
    };
    ($group_kind:ident, $input:expr, $ident:expr) => {
        {
            let id: syn::Ident = $input.parse().unwrap_or_else(|e| non_user_error!(e));
            let ident = $ident;
            if id != ident {
                non_user_error!(format!("expected `{ident}`"));
            }
            non_user_group! { $group_kind, $input }
        }
    }
}
/// Does a `braced!` parse but panics with [`non_user_error!()`](non_user_error) if the parsing fails.
macro_rules! non_user_braced {
    ($input:expr) => {
        non_user_group! { braced, $input }
    };
    ($input:expr, $ident:expr) => {
        non_user_group! { braced, $input, $ident }
    };
}
