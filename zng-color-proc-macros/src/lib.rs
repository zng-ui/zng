#![doc = include_str!("../../zng-app/README.md")]
//!
//! Proc-macros for `zng-color`.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

#[macro_use]
extern crate quote;

mod hex_color;

#[doc(hidden)]
#[proc_macro]
pub fn hex_color(input: TokenStream) -> TokenStream {
    hex_color::expand(input)
}
