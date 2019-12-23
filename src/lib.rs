#![warn(unused_extern_crates)]

#[macro_use]
extern crate derive_new;

#[macro_use]
mod macros;

pub use zero_ui_macros::{impl_ui, ui_property, ui_widget};

use proc_macro_hack::proc_macro_hack;

#[doc(hidden)]
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::custom_ui;

/// Defines an item widget made entirely of property behavior.
///
/// # Arguments
/// * `id`: Sets the item id to a custom value. By default generates a new id.
/// * `ui properties`: All ui properties can be set.
///
/// # Example
/// ```
/// # use zero_ui::{ui, primitive::*};
/// #
/// let item = ui! {
///     font_family: "serif";
///     font_size: 22;
///
///     => text("Hello!")
/// };
/// ```
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::ui;

/// Defines a part of an widget without turning it into a full item by setting an `id`.
///
/// # Arguments
/// * `ui properties`: All ui properties can be set.
///
/// # Example
/// ```
/// # use zero_ui::{ui_part, primitive::*};
/// # let message = "message";
/// #
/// let msg_part = ui_part! {
///     text_color: rgb(0, 180, 0);
///
///     => text(message)
/// };
/// ```
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::ui_part;

pub mod core;
pub mod primitive;
pub mod widget;

pub mod app;

pub mod test;
