#![warn(unused_extern_crates)]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```no_run
//! use zero_ui::prelude::*;
//!
//! fn main () {
//!     App::default().run_window(|_| {
//!         let size = var((800., 600.));
//!         let title = size.map(|s: &LayoutSize| formatx!("Button Example - {}x{}", s.width.ceil(), s.height.ceil()));
//!         window! {
//!             size;
//!             title;
//!             content: example();
//!         }
//!     })
//! }
//!
//! fn example() -> impl UiNode {
//!     button! {
//!         on_click: |_| {
//!             println!("Button clicked!");
//!         };
//!         margin: 10.0;
//!         size: (300.0, 200.0);
//!         align: Alignment::CENTER;
//!         font_size: 28;
//!         content: text("Click Me!");
//!     }
//! }
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
pub use zero_ui_macros::{widget_new, widget_stage2, widget_stage3};

pub mod core;
pub mod properties;
pub mod widgets;

/// All the types you need to build an app.
pub mod prelude {
    pub use crate::core::{
        app::App,
        types::{
            formatx, rgb, rgba, BorderRadius, ColorF, CursorIcon, ElementState, FontName, FontSize, FontStretch, FontStyle, FontWeight,
            LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, ModifiersState, MouseButton, Text, ToText, VirtualKeyCode, WidgetId,
        },
        ui_vec,
        var::{var, SharedVar, Var},
        window::{AppRunWindow, Window, Windows},
        UiNode, UiVec, Widget,
    };
    pub use crate::properties::*;
    pub use crate::widgets::layouts::*;
    pub use crate::widgets::*;
}
