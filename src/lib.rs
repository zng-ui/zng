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
#[allow(unused)]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

/// Calls `eprintln!("warning: {}", format_args!($))` with `warning` colored bright yellow and bold.
#[allow(unused)]
macro_rules! warn_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "warning".bright_yellow().bold(), format_args!($($tt)*))
    }}
}

/// Declare a new unique id type.
macro_rules! unique_id {
    ($(#[$docs:meta])* $vis:vis $Type:ident;) => {

        $(#[$docs])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU64);

        impl $Type {
            fn next() -> &'static std::sync::atomic::AtomicU64 {
                use std::sync::atomic::AtomicU64;
                static NEXT: AtomicU64 = AtomicU64::new(1);
                &NEXT
            }

            /// Generates a new unique ID.
            ///
            /// # Panics
            /// Panics if called more then `u64::MAX` times.
            pub fn new_unique() -> Self {
                use std::sync::atomic::Ordering;

                let id = Self::next().fetch_add(1, Ordering::Relaxed);

                if let Some(id) = std::num::NonZeroU64::new(id) {
                    $Type(id)
                } else {
                    Self::next().store(0, Ordering::SeqCst);
                    panic!("`{}` reached `u64::MAX` IDs.", stringify!($Type))
                }
            }

            /// Retrieve the underlying `u64` value.
            #[allow(dead_code)]
            #[inline]
            pub fn get(self) -> u64 {
                self.0.get()
            }

            /// Creates an id from a raw value.
            ///
            /// # Safety
            ///
            /// This is only safe if called with a value provided by [`get`](Self::get).
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u64) -> $Type {
                $Type(std::num::NonZeroU64::new_unchecked(raw))
            }

            /// Creates an id from a raw value.
            ///
            /// Checks if `raw` is in the range of generated widgets.
            #[inline]
            #[allow(dead_code)]
            pub fn new(raw: u64) -> Option<$Type> {
                use std::sync::atomic::Ordering;

                if raw >= 1 && raw < Self::next().load(Ordering::Relaxed) {
                    // SAFETY: we just validated raw.
                    Some(unsafe { Self::from_raw(raw) })
                } else {
                    None
                }
            }
        }
    };
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
        focus::{DirectionalNav, TabIndex, TabNav},
        gesture::shortcut,
        render::WidgetPath,
        types::{
            formatx, rgb, rgba, rotate, BorderRadius, ColorF, CursorIcon, ElementState, FontName, FontSize, FontStretch, FontStyle,
            FontWeight, LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, ModifiersState, MouseButton, Text, ToText, VirtualKeyCode,
            WidgetId,
        },
        ui_vec,
        var::{merge_var, switch_var, var, SharedVar, Var},
        window::{AppRunWindow, Window, Windows},
        UiNode, UiVec, Widget,
    };
    pub use crate::properties::*;
    pub use crate::widgets::layouts::*;
    pub use crate::widgets::*;
}
