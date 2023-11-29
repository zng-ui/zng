use proc_macro2::{TokenStream, TokenTree, Span};
use quote::ToTokens;

/// Returns `true` if `a` and `b` have the same tokens in the same order (ignoring span).
pub fn token_stream_eq(a: TokenStream, b: TokenStream) -> bool {
    let mut a = a.into_iter();
    let mut b = b.into_iter();
    use TokenTree::*;
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => match (a, b) {
                (Group(a), Group(b)) if a.delimiter() == b.delimiter() && token_stream_eq(a.stream(), b.stream()) => continue,
                (Ident(a), Ident(b)) if a == b => continue,
                (Punct(a), Punct(b)) if a.as_char() == b.as_char() && a.spacing() == b.spacing() => continue,
                (Literal(a), Literal(b)) if a.to_string() == b.to_string() => continue,
                _ => return false,
            },
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Generate a [`String`] that is a valid [`Ident`] from an arbitrary [`TokenStream`].
pub fn tokens_to_ident_str(tokens: &TokenStream) -> String {
    let tokens = tokens.to_string();
    let max = tokens.len().min(40);
    let mut tokens = tokens[(tokens.len() - max)..]
        .replace(&['.', ':', ' '][..], "_")
        .replace('!', "not")
        .replace("&&", "and")
        .replace("||", "or")
        .replace('(', "p")
        .replace(')', "b")
        .replace("==", "eq");

    tokens.retain(|c| c == '_' || c.is_alphanumeric());

    tokens
}


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


const RECOVERABLE_TAG: &str = "<recoverable>";
fn recoverable_tag() -> syn::Error {
    syn::Error::new(Span::call_site(), RECOVERABLE_TAG)
}

/// Extension to [`syn::Error`] that lets you mark an error as recoverable,
/// meaning that a sequence of the parse stream is not correct but the parser
/// manage to skip to the end of what was expected to be parsed.
pub trait ErrorRecoverable {
    /// Returns a new error that contains all the errors in `self` but is also marked recoverable.
    fn set_recoverable(self) -> Self;
    /// Returns if `self` is recoverable and all the errors in `self`.
    ///
    /// Note: An error is considered recoverable only if all inner errors are marked recoverable.
    fn recoverable(self) -> (bool, Self);
}
impl ErrorRecoverable for syn::Error {
    fn set_recoverable(self) -> Self {
        let mut errors = self.into_iter();
        let mut e = errors.next().unwrap();

        debug_assert!(e.to_string() != RECOVERABLE_TAG);

        e.combine(recoverable_tag());

        for error in errors {
            if e.to_string() != RECOVERABLE_TAG {
                e.combine(error);
                e.combine(recoverable_tag());
            }
        }

        e
    }
    fn recoverable(self) -> (bool, Self) {
        let mut errors = self.into_iter();
        let mut e = errors.next().unwrap();

        debug_assert!(e.to_string() != RECOVERABLE_TAG);

        let mut errors_count = 1;
        let mut tags_count = 0;

        for error in errors {
            if error.to_string() == RECOVERABLE_TAG {
                tags_count += 1;
            } else {
                errors_count += 1;
                e.combine(error);
            }
        }

        (errors_count == tags_count, e)
    }
}

/// Generates a return of a compile_error message in the given span.
macro_rules! abort {
    ($span:expr, $($tt:tt)*) => {{
        let error = format!($($tt)*);
        let error = syn::LitStr::new(&error, proc_macro2::Span::call_site());

        return quote_spanned!($span=> compile_error!{#error}).into();
    }};
}

/// Generates a return of a compile_error message in the call_site span.
macro_rules! abort_call_site {
    ($($tt:tt)*) => {
        abort!(proc_macro2::Span::call_site(), $($tt)*)
    };
}

/// Input error not caused by the user.
macro_rules! non_user_error {
    ($e:expr) => {
        panic!("[{}:{}] invalid non-user input: {}", file!(), line!(), $e)
    };
    ($fmt:tt, $($args:tt)+) => {
        non_user_error! { format_args!($fmt, $($args)+) }
    }
}

/// `Ident` with custom span.
macro_rules! ident_spanned {
    ($span:expr=> $($format_name:tt)+) => {
        proc_macro2::Ident::new(&format!($($format_name)+), $span)
    };
}

/// `Ident` with call_site span.
macro_rules! ident {
    ($($tt:tt)*) => {
        ident_spanned!(proc_macro2::Span::call_site()=> $($tt)*)
    };
}