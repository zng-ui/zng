//! Common widget properties.

mod util;
pub use util::*;

mod layout;
pub use layout::*;

mod visual;
pub use visual::*;

mod border_;
pub use border_::*;

pub mod commands;
pub mod drag_move;
pub mod events;
pub mod filters;
pub mod focus;
pub mod states;
pub mod transform;

mod capture;
pub use capture::*;
mod cursor_;
pub use cursor_::*;

#[doc(no_inline)]
pub use crate::core::widget_base::{hit_testable, interactive};
