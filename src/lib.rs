#[macro_use]
extern crate derive_new;

pub use zero_ui_macros::{impl_ui, profile_scope};

use proc_macro_hack::proc_macro_hack;
#[proc_macro_hack]
pub use zero_ui_macros::ui;

#[macro_use]
pub mod core;
pub mod primitive;

pub mod app;
