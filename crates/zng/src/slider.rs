//! Slider widget, styles and properties.
//!
//! This widget allows selecting a value or range by dragging a selector thumb over a range line.
//!
//! ```
//! // !!: TODO
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_slider`] for the full widget API.

pub use zng_wgt_slider::{DefaultStyle, Selector, Slider, SliderDirection, SliderTrack, ThumbArgs, SLIDER_DIRECTION_VAR};

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
