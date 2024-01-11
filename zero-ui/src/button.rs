//! Button widget, styles and properties.
//!
//! A simple clickable widget.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! # let _scope = APP.defaults();
//! let count = var(0u32);
//! # let _ =
//! Button! {
//!     child = Text!(count.map(|c| match *c {
//!         0 => Txt::from("Click Me!"),
//!         n => formatx!("Clicked {n} times."),
//!     }));
//!     on_click = hn!(|_| {
//!         count.set(count.get() + 1);
//!     });
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zero_ui_wgt_button`] for the full widget API.

pub use zero_ui_wgt_button::{base_colors, extend_style, replace_style, Button, DefaultStyle};

pub use zero_ui_wgt_link::LinkStyle;
