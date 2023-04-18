//! Common widget properties.

pub mod inspector;

mod layout;
#[doc(inline)]
pub use layout::*;

mod visual;
#[doc(inline)]
pub use visual::*;

mod border_;
#[doc(inline)]
pub use border_::*;

pub mod commands;
pub mod events;
pub mod filters;
pub mod focus;
pub mod states;
pub mod transform;

mod capture;
#[doc(inline)]
pub use capture::*;

mod mouse;
#[doc(inline)]
pub use mouse::*;

mod lazy_;
#[doc(inline)]
pub use lazy_::*;

mod tooltip_;
#[doc(inline)]
pub use tooltip_::*;

#[doc(inline)]
pub use crate::core::widget_base::{enabled, hit_test_mode, interactive};
