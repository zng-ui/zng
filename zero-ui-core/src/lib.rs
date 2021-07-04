#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]
#![cfg_attr(doc_nightly, feature(doc_cfg))]
#![cfg_attr(doc_nightly, feature(doc_notable_trait))]

//! Core infrastructure required for creating components and running an app.

#[macro_use]
extern crate bitflags;

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui_core;

#[macro_use]
mod crate_util;

#[doc(hidden)]
pub use paste::paste;

pub mod animation;
pub mod app;
pub mod border;
pub mod color;
pub mod command;
pub mod context;
pub mod debug;
pub mod event;
#[macro_use]
pub mod handler;
pub mod focus;
pub mod gesture;
pub mod gradient;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod service;
pub mod task;
pub mod text;
pub mod timer;
pub mod units;
pub mod var;
pub mod widget_base;
pub mod window;

mod ui_node;
pub use ui_node::*;

mod ui_list;
pub use ui_list::*;

// proc-macros used internally during widget creation.
#[doc(hidden)]
pub use zero_ui_proc_macros::{property_new, widget_declare, widget_inherit, widget_new};

/// Gets if the value indicates that any size is available during layout (positive infinity)
// TODO move to units
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A map of TypeId -> Box<dyn UnsafeAny>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn unsafe_any::UnsafeAny>>;

pub use zero_ui_proc_macros::{impl_ui_node, property, widget, widget_mixin};

mod tests;
