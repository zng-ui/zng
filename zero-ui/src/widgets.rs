//! Common widgets.

pub mod layouts;

pub mod focusable;

pub mod ansi_text;
#[doc(inline)]
pub use ansi_text::AnsiText;

pub mod button;
#[doc(inline)]
pub use button::Button;

pub mod checkerboard;
#[doc(inline)]
pub use checkerboard::Checkerboard;

mod container;
#[doc(inline)]
pub use container::Container;

mod flood;
#[doc(inline)]
pub use flood::flood;

mod gradient;
#[doc(inline)]
pub use gradient::{
    conic_gradient, conic_gradient_ext, conic_gradient_full, linear_gradient, linear_gradient_ext, linear_gradient_full, radial_gradient,
    radial_gradient_ext, radial_gradient_full, reflecting_conic_gradient, reflecting_linear_gradient, reflecting_radial_gradient,
    repeating_conic_gradient, repeating_linear_gradient, repeating_radial_gradient,
};

pub mod image;
#[doc(inline)]
pub use image::Image;

pub mod icon;
#[doc(inline)]
pub use icon::Icon;

pub mod link;
#[doc(inline)]
pub use link::Link;

mod markdown;
#[doc(inline)]
pub use markdown::Markdown;

pub mod rule_line;
#[doc(inline)]
pub use rule_line::{hr::Hr, RuleLine};

pub mod scroll;
#[doc(inline)]
pub use scroll::Scroll;

pub mod switch;
#[doc(inline)]
pub use switch::Switch;

pub mod text;
#[doc(inline)]
pub use text::{Em, Strong, Text};

pub mod text_input;
#[doc(inline)]
pub use text_input::TextInput;

pub mod tip;
#[doc(inline)]
pub use tip::Tip;

pub mod toggle;
#[doc(inline)]
pub use toggle::Toggle;

pub mod style;
#[doc(inline)]
pub use style::Style;

mod view;
#[doc(inline)]
pub use view::*;

mod window_wgt;
#[doc(inline)]
pub use window_wgt::window;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget that is only used in one code place and can be created entirely
/// by properties and `when` conditions.
#[crate::core::widget($crate::widgets::Wgt)]
pub struct Wgt(crate::core::widget_base::WidgetBase);
