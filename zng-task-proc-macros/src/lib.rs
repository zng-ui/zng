#![doc = include_str!("../../zng-app/README.md")]
//!
//! Proc-macros for `zng-task`.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

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