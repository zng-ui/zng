//! Checkerboard visual widget.
//!
//! The widget appearance can be configured on it or in any parent widget, by default it looks like
//! the transparency checkerboard.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! # let _scope = APP.defaults();
//! zero_ui::image::IMAGES.limits().modify(|l| {
//!     l.to_mut().allow_uri = zero_ui::image::UriFilter::AllowAll;
//! });
//!
//! # let _ =
//! Image! {
//!     widget::background = zero_ui::checkerboard::Checkerboard!();
//!     source = "https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png";
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zero_ui_wgt_checkerboard`] for the full widget API.

pub use zero_ui_wgt_checkerboard::{cb_origin, cb_size, colors, Checkerboard, Colors};
