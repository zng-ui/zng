use proc_macro::TokenStream;

#[macro_use]
extern crate quote;

#[macro_use]
mod util;

mod any_all;

#[doc(hidden)]
#[proc_macro]
pub fn task_any_all(input: TokenStream) -> TokenStream {
    any_all::expand(input)
}
