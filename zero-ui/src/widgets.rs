//! Common widgets.

pub mod layouts;

pub mod focusable_mix;
pub mod undo_mix;

pub mod ansi_text;
pub use ansi_text::AnsiText;

pub mod button;
pub use button::Button;

pub mod checkerboard;
pub use checkerboard::Checkerboard;

mod container;
pub use container::Container;

mod flood;
pub use flood::flood;

pub mod gradient;
pub use gradient::{conic_gradient, gradient, linear_gradient, radial_gradient};

pub mod image;
pub use image::Image;

pub mod icon;
pub use icon::Icon;

pub mod markdown;
pub use markdown::Markdown;

pub mod rule_line;
pub use rule_line::{hr::Hr, vr::Vr, RuleLine};

pub mod scroll;
pub use scroll::Scroll;

pub mod switch;
pub use switch::Switch;

pub mod text;
pub use text::{Em, Strong, Text};

pub mod text_input;
pub use text_input::TextInput;

pub mod tip;
pub use tip::Tip;

pub mod toggle;
pub use toggle::Toggle;

pub mod style;
pub use style::Style;

mod view;
pub use view::*;

pub mod window;
pub use window::Window;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget that is only used in one code place and can be created entirely
/// by properties and `when` conditions.
#[crate::core::widget($crate::widgets::Wgt)]
pub struct Wgt(crate::core::widget_base::WidgetBase);
