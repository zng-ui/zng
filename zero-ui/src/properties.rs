//! Common widget properties.

pub mod inspector;

mod layout;
pub use layout::*;

mod visual;
pub use visual::*;

mod border_;
pub use border_::*;

pub mod commands;

mod data_;
pub use data_::*;

pub mod events;
pub mod filters;
pub mod focus;
pub mod states;
pub mod transform;

mod undo;
pub use undo::*;

mod capture;
pub use capture::*;

mod mouse;
pub use mouse::*;

mod lazy_;
pub use lazy_::*;

mod tooltip_;
pub use tooltip_::*;

pub use crate::core::widget_base::{enabled, hit_test_mode, interactive};
