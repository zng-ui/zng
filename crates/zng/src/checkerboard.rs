#![cfg(feature = "checkerboard")]

//! Checkerboard visual widget.
//!
//! The widget appearance can be configured on it or in any parent widget, by default it looks like
//! the transparency checkerboard.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! zng::image::IMAGES.limits().modify(|l| {
//!     l.allow_uri = zng::image::UriFilter::AllowAll;
//! });
//!
//! # let _ =
//! Image! {
//!     widget::background = zng::checkerboard::Checkerboard!();
//!     source = "https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png";
//! }
//! # ; }
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_checkerboard`] for the full widget API.

pub use zng_wgt_checkerboard::{Checkerboard, Colors, cb_origin, cb_size, colors};
