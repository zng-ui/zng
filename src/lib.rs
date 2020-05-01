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

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

#[doc(hidden)]
pub use zero_ui_macros::{widget_inherit, widget_mixin_inherit, widget_new};

pub mod core;
pub mod layouts;
pub mod properties;
pub mod widgets;

/// All the types you need to build an app.
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
