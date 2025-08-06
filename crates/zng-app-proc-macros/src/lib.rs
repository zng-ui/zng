#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Proc-macros for `zng-app`.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

#[macro_use]
extern crate quote;

#[macro_use]
extern crate lazy_static;

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod property;
mod wgt_property_attrs;
mod widget;
mod widget_util;

/// Expands a function to a widget property.
///
/// # Full Documentation
///
/// Read the documentation in the `zng::widget::property` page.
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn property_meta(input: TokenStream) -> TokenStream {
    property::expand_meta(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn property_impl(input: TokenStream) -> TokenStream {
    property::expand_impl(input)
}

/// Expands a struct to a widget and macro.
///
/// # Full Documentation
///
/// Read the documentation in the `zng::widget::widget` page.
#[proc_macro_attribute]
pub fn widget(args: TokenStream, input: TokenStream) -> TokenStream {
    widget::expand(args, input, false)
}

/// Expands a generic struct to a widget mixin.
///
/// # Full Documentation
///
/// Read the documentation in the `zng::widget::widget_mixin` page.
#[proc_macro_attribute]
pub fn widget_mixin(args: TokenStream, input: TokenStream) -> TokenStream {
    widget::expand(args, input, true)
}

/// Expands a property assign to include a build action that applies an easing transition to the variable inputs.
///
/// # Full Documentation
///
/// Read the documentation in the `zng::widget::easing`.
#[proc_macro_attribute]
pub fn easing(args: TokenStream, input: TokenStream) -> TokenStream {
    wgt_property_attrs::expand_easing(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget::expand_new(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    eprintln!("{input}");
    input
}
