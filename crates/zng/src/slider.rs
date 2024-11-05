//! Slider widget, styles and properties.
//!
//! This widget allows selecting a value or range by dragging a selector thumb over a range line.
//!
//! ```
//! todo!()
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_slider`] for the full widget API.

pub use zng_wgt_slider::{Selector, Slider};

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
