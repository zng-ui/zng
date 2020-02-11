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

extern crate self as zero_ui;

#[macro_use]
mod macros;

pub use zero_ui_macros::{impl_ui_node, property, widget};

use proc_macro_hack::proc_macro_hack;

#[doc(hidden)]
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::widget_new;

pub mod core;
pub mod layouts;
pub mod properties;
pub mod widgets;

pub mod prelude {
    pub use crate::core::{
        types::{rgb, rgba},
        var::var,
    };
    pub use crate::layouts::*;
    pub use crate::properties::*;
    pub use crate::widgets::*;
}
