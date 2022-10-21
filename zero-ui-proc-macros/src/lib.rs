//! [`zero-ui`](../zero_ui/index.html) proc-macros.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod derive_service;
pub(crate) mod expr_var;
mod hex_color;
mod merge_var;
mod when_var;

mod property;
mod ui_node;
mod widget;
mod widget_util;

mod any_all;

mod lang;

/// Expands an impl into a `UiNode` impl.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::ui_node`](../zero_ui_core/attr.ui_node.html) page.
#[proc_macro_attribute]
pub fn ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    ui_node::gen_ui_node(args, input)
}

/// Expands a function to a widget property.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::property`](../zero_ui_core/attr.property.html) page.
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn hex_color(input: TokenStream) -> TokenStream {
    hex_color::expand(input)
}

#[doc(hidden)]
#[proc_macro_derive(Service)]
pub fn derive_service(item: TokenStream) -> TokenStream {
    derive_service::derive(item)
}

/// Expands a module to a widget module and macro.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::widget`](../zero_ui_core/attr.widget.html) page.
#[proc_macro_attribute]
pub fn widget(args: TokenStream, input: TokenStream) -> TokenStream {
    widget::expand(args, input, false)
}

/// Expands a module to a widget mix-in module and macro.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::widget_mixin`](../zero_ui_core/attr.widget_mixin.html) page.
#[proc_macro_attribute]
pub fn widget_mixin(args: TokenStream, input: TokenStream) -> TokenStream {
    widget::expand(args, input, true)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget::expand_new(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn expr_var(input: TokenStream) -> TokenStream {
    expr_var::expand(input)
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

#[doc(hidden)]
#[proc_macro]
pub fn task_any_all(input: TokenStream) -> TokenStream {
    any_all::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    eprintln!("{input}");
    input
}

#[doc(hidden)]
#[proc_macro]
pub fn lang(input: TokenStream) -> TokenStream {
    lang::expand(input)
}
