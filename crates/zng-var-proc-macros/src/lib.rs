#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
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

mod expr_var;
mod impl_property_value;
mod merge_var;
mod transitionable;
mod when_var;

/// Implement transition by delegating all type parts.
#[proc_macro_derive(Transitionable)]
pub fn transitionable(args: TokenStream) -> TokenStream {
    transitionable::expand(args)
}

#[doc(hidden)]
#[proc_macro]
pub fn expr_var(input: TokenStream) -> TokenStream {
    expr_var::expand(input)
}

/// Implement `PropertyValue`.
#[proc_macro_attribute]
pub fn impl_property_value(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_property_value::expand(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn when_var(input: TokenStream) -> TokenStream {
    when_var::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn merge_var(input: TokenStream) -> TokenStream {
    merge_var::expand(input)
}
