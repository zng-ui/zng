//! Slider widget, styles and properties.
//!
//! This widget allows selecting a value or range by dragging a selector thumb over a range line.
//!
//! ```
//! # use zng::prelude::*;
//! let value = var(0u8);
//! # let _ =
//! zng::slider::Slider! {
//!     selector = zng::slider::Selector::value(value.clone(), 0, 100);
//! }
//! ```
//!
//! The example above creates a a slider with a single thumb that selects a `u8` value in the `0..=100` range.
//!
//! # Full API
//!
//! See [`zng_wgt_slider`] for the full widget API.

pub use zng_wgt_slider::{DefaultStyle, Selector, SelectorValue, Slider, SliderDirection, SliderTrack, ThumbArgs, SLIDER_DIRECTION_VAR};

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
