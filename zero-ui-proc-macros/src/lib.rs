//! [`zero-ui`](../zero_ui_proc_macros/index.html) proc-macros.

extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod derive_service;
pub(crate) mod expr_var;
mod hex_color;
mod when_var;

mod impl_ui_node;
pub(crate) mod property;

mod widget_0_attr;
mod widget_1_inherit;
mod widget_2_declare;

mod property_new;
mod widget_new;

/// Expands an impl into a `UiNode` impl.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui::core::impl_ui_node`](../zero_ui/core/attr.impl_ui_node.html) page.
#[proc_macro_attribute]
pub fn impl_ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_node::gen_impl_ui_node(args, input)
}

/// Expands a function to a widget property.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui::core::property`](../zero_ui/core/attr.property.html) page.
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand(args, input)
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
#[proc_macro_derive(AppService)]
pub fn derive_app_service(item: TokenStream) -> TokenStream {
    derive_service::derive(item, ident!("AppService"))
}

#[doc(hidden)]
#[proc_macro_derive(WindowService)]
pub fn derive_window_service(item: TokenStream) -> TokenStream {
    derive_service::derive(item, ident!("WindowService"))
}

/// Expands a module to a widget module and macro.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui::core::widget2`](../zero_ui/core/attr.widget2.html) page.
#[proc_macro_attribute]
pub fn widget(args: TokenStream, input: TokenStream) -> TokenStream {
    widget_0_attr::expand(false, args, input)
}

/// Expands a module to a widget mix-in module.
///
/// # Full Documentation
///
/// Read the documentation in the [`zero_ui::core::widget_mixin2`](../zero_ui/core/attr.widget_mixin2.html) page.
#[proc_macro_attribute]
pub fn widget_mixin(args: TokenStream, input: TokenStream) -> TokenStream {
    widget_0_attr::expand(true, args, input)
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
