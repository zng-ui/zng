#![cfg(feature = "slider")]

//! Slider widget, styles and properties.
//!
//! This widget allows selecting a value or range by dragging a selector thumb over a range line.
//!
//! ```
//! # use zng::prelude::*;
//! # fn example() {
//! let value = var(0u8);
//! # let _ =
//! zng::slider::Slider! {
//!     // declare slider with single thumb
//!     selector = zng::slider::Selector::value(value.clone(), 0, 100);
//!     // show selected value
//!     zng::container::child_out_bottom = Text!(value.map_debug(false)), 5;
//! }
//! # ; }
//! ```
//!
//! The example above creates a a slider with a single thumb that selects a `u8` value in the `0..=100` range. The [`Selector`]
//! type also supports creating multiple thumbs and custom range conversions.
//!
//! # Full API
//!
//! See [`zng_wgt_slider`] for the full widget API.

pub use zng_wgt_slider::{DefaultStyle, SLIDER_DIRECTION_VAR, Selector, SelectorValue, Slider, SliderDirection, SliderTrack, ThumbArgs};

/// Slider thumb widget, styles and properties.
///
/// This widget represents one value/offset in a slider.
///
/// # Full API
///
/// See [`zng_wgt_slider::thumb`] for the full widget API.
pub mod thumb {
    pub use zng_wgt_slider::thumb::{DefaultStyle, Thumb};
}
