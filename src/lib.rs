#![warn(unused_extern_crates)]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```
//! #[macro_use]
//! extern crate zero_ui;
//!
//! use zero_ui::prelude::*;
//! ```

#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate bitflags;

extern crate self as zero_ui;

#[macro_use]
mod macros;

pub use zero_ui_macros::{impl_ui_node, property, widget};

use proc_macro_hack::proc_macro_hack;

#[doc(hidden)]
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::widget_new;

// old macros
#[doc(hidden)]
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::custom_ui;
pub use zero_ui_macros::{impl_ui, ui_property, ui_widget};

/// Defines an item widget made entirely of property behavior.
///
/// # Arguments
/// * `id`: Sets the item id to a custom value. By default generates a new id.
/// * `ui properties`: All ui properties can be set.
///
/// # Example
/// ```
/// # use zero_ui::{ui, properties::*};
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
/// # use zero_ui::{ui_part, properties::*};
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
pub mod core2;
pub mod layouts;
pub mod properties;
pub mod widgets;

pub mod app;

pub mod test;

pub mod prelude {
    pub use crate::app;
    pub use crate::core::*;
    pub use crate::layouts::*;
    pub use crate::properties::*;
    pub use crate::widgets::*;
}
