#![warn(unused_extern_crates)]
#![recursion_limit = "256"]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```
//! #[macro_use]
//! extern crate zero_ui;
//!
//! use zero_ui::prelude::*;
//!
//! # fn main () {}
//! ```

// for proc_macros that don't have $self.
extern crate self as zero_ui;

#[macro_use]
mod macros;

pub use zero_ui_macros::{impl_ui_node, property, widget, widget_mixin};
#[doc(hidden)]
pub use zero_ui_macros::{widget_inherit, widget_mixin_inherit};

use proc_macro_hack::proc_macro_hack;

#[doc(hidden)]
#[proc_macro_hack(support_nested)]
pub use zero_ui_macros::widget_new;

#[macro_use]
pub mod core;
pub mod layouts;
pub mod properties;
pub mod widgets;

pub mod prelude {
    pub use crate::core::{
        app::App,
        types::{
            rgb, rgba, BorderRadius, ColorF, CursorIcon, ElementState, LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize,
            ModifiersState, MouseButton, Text, ToText, VirtualKeyCode, WidgetId,
        },
        var::var,
        window::{Window, Windows},
        UiNode,
    };
    pub use crate::layouts::*;
    pub use crate::properties::*;
    pub use crate::widgets::*;
}
