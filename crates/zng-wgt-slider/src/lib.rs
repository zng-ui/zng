#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Widget for selecting a value or range by dragging a selector knob.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;

/// Value selector from a range of values.
#[widget($crate::Slider)]
pub struct Slider(WidgetBase);

/*
!!: TODO

* Single value selector.
* Range selector.
* Snapping value options.
* Full numeric range options.
* Different intervals for options (like some options closer together).
* Different "importance" of options, like some options with a larger marker.

*/