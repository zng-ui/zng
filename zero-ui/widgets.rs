//! Common widgets.

pub mod layouts;
pub mod mixins;

mod button_;
mod checkerboard_;
mod container_;
mod fill_color;
mod gradient;
mod image_;
mod line_;
mod scrollable_;
mod slot_;
mod switch_;
mod text_;
mod ui_n;
mod view_;
mod window_;

pub use button_::*;
pub use checkerboard_::*;
pub use container_::*;
pub use fill_color::*;
pub use gradient::*;
pub use image_::*;
pub use line_::*;
pub use scrollable_::*;
pub use slot_::*;
pub use switch_::*;
pub use text_::*;
pub use ui_n::*;
pub use view_::*;
pub use window_::*;

/// A widget with only the implicit properties.
///
/// You can use this to shape a custom widget that will only
/// be used once. Instead of declaring a new widget type.
#[crate::core::widget($crate::widgets::blank)]
pub mod blank {}
