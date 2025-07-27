#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Proc-macros for the `zng-var` crate.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

#[macro_use]
extern crate quote;

#[macro_use]
mod util;

mod transitionable;
mod var_expr;
mod var_merge;
mod var_when;

/// Implement transition by delegating all type parts.
#[proc_macro_derive(Transitionable)]
pub fn transitionable(args: TokenStream) -> TokenStream {
    transitionable::expand(args)
}

#[doc(hidden)]
#[proc_macro]
pub fn var_expr(input: TokenStream) -> TokenStream {
    var_expr::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn var_when(input: TokenStream) -> TokenStream {
    var_when::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn var_merge(input: TokenStream) -> TokenStream {
    var_merge::expand(input)
}
