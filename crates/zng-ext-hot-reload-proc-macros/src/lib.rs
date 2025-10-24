#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Proc-macros for `zng-ext-hot-reload`.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod hot_node;

/// Expands an impl into a `UiNode` impl.
///
/// # Full Documentation
///
/// Read the documentation in the `zng::hot_reload::hot_node` page.
#[proc_macro_attribute]
pub fn hot_node(args: TokenStream, input: TokenStream) -> TokenStream {
    hot_node::expand(args, input)
}
