#[macro_use]
extern crate derive_new;

#[macro_use]
mod macros;

pub use zero_ui_macros::{impl_ui, ui_property};

use proc_macro_hack::proc_macro_hack;
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::ui;

pub mod core;
pub mod primitive;
pub mod widget;

pub mod app;
