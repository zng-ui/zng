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
