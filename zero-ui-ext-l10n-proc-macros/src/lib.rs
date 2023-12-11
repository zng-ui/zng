//! Proc-macros for `zero-ui-ext-l10n`, don't use directly.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod l10n;
mod lang;

#[doc(hidden)]
#[proc_macro]
pub fn l10n(input: TokenStream) -> TokenStream {
    l10n::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn lang(input: TokenStream) -> TokenStream {
    lang::expand(input)
}
