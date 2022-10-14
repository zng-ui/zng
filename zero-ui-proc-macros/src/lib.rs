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
mod static_list;
mod when_var;

pub(crate) mod property;
mod property2;
mod ui_node;
pub(crate) mod widget_util;

mod widget_0_attr;
mod widget_1_inherit;
mod widget_2_declare;

mod property_new;
mod widget_new;

mod any_all;

mod lang;
mod rust_analyzer;
mod widget2;

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

/// Expands a function to a widget property.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::property`](../zero_ui_core/attr.property.html) page.
#[proc_macro_attribute]
pub fn property2(args: TokenStream, input: TokenStream) -> TokenStream {
    property2::expand(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn property_new(input: TokenStream) -> TokenStream {
    property_new::expand(input)
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
    widget_0_attr::expand(false, false, args, input)
}

/// Expands a module to a widget module and macro.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::widget`](../zero_ui_core/attr.widget.html) page.
#[proc_macro_attribute]
pub fn widget2(args: TokenStream, input: TokenStream) -> TokenStream {
    widget2::expand(args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_new2(input: TokenStream) -> TokenStream {
    widget2::expand_new(input)
}

// used only once to declare the widget base.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn widget_base(args: TokenStream, input: TokenStream) -> TokenStream {
    widget_0_attr::expand(false, true, args, input)
}

/// Expands a module to a widget mix-in module.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui_core::widget_mixin`](../zero_ui_core/attr.widget_mixin.html) page.
#[proc_macro_attribute]
pub fn widget_mixin(args: TokenStream, input: TokenStream) -> TokenStream {
    widget_0_attr::expand(true, false, args, input)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_inherit(input: TokenStream) -> TokenStream {
    widget_1_inherit::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_declare(input: TokenStream) -> TokenStream {
    widget_2_declare::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand(input)
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
pub fn static_list(input: TokenStream) -> TokenStream {
    static_list::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn lang(input: TokenStream) -> TokenStream {
    lang::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn rust_analyzer_widget_new(input: TokenStream) -> TokenStream {
    rust_analyzer::widget_new(input)
}
