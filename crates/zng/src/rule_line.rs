#![cfg(feature = "rule_line")]

//! Rule line widgets and properties.
//!
//! A rule line is a horizontal or vertical separator line, this module provides 3 widgets the [`RuleLine!`](struct@RuleLine)
//! base that can dynamically change orientation and the [`hr::Hr!`](struct@hr::Hr) and [`vr::Vr!`](struct@vr::Vr) that represents
//! each orientation and can be styled separately.
//!
//! ```
//! use zng::prelude::*;
//! # fn demo() {
//!
//! # let _ =
//! Window! {
//!     context_menu = ContextMenu!(ui_vec![
//!         Button!(zng::app::NEW_CMD.scoped(WINDOW.id())),
//!         Button!(zng::app::OPEN_CMD.scoped(WINDOW.id())),
//!         Hr!(),
//!         Button!(zng::app::EXIT_CMD),
//!     ]);
//! }
//! # ; }
//! ```
//!
//! The example above uses the `Hr!` widget in a context menu to separate the commands into two groups.
//!
//! # Collapse Scope
//!
//! Sometimes two or more separator lines can end-up appearing adjacent to one another, not actually *separating* anything. A
//! parent panel widget can set [`collapse_scope`] to automatically *trim* or *merge* separator lines in its descendants.
//!
//! The `ContextMenu!`, `Menu!` and `SubMenu!` widgets enable this feature by default, the standalone property can also be set in any other widget.
//!
//! ```
//! # use zng::prelude::*;
//! # let _ =
//! Wrap! {
//!     id = "toolbar";
//!     zng::rule_line::vr::height = 1.em();
//!     zng::rule_line::collapse_scope = true;
//!     children = ui_vec![
//!         Button!(zng::app::OPEN_CMD.scoped(WINDOW.id())),
//!         zng::rule_line::vr::Vr!(),
//!         Button!(zng::clipboard::COPY_CMD.scoped("content")),
//!         Button!(zng::clipboard::PASTE_CMD.scoped("content")),
//!     ];
//! }
//! # ;
//! ```
//!
//! The example above defines a `"toolbar"` panel with a vertical separator, command buttons are not visible when the command has no handle,
//! in the example the clipboard commands are scoped to a `"content"` target, if that widget does not exist the buttons will collapse so the
//! `Vr!()` would appear dangling at the end. In this example toolbar enables all features of [`collapse_scope`] that includes [`CollapseMode::TRIM_END`],
//! so the vertical line will collapse as well, until the `"content"` widget is loaded.
//!
//! Note that [`collapse_scope`] also works in nested panels, a more complex *toolbars* setup can enable it at the *toolbar tray* root widget and
//! all *toolbar* widgets can be dynamically moved and the separator lines will collapse as needed.
//!
//! [`collapse_scope`]: fn@collapse_scope
//!
//! # Full API
//!
//! See [`zng_wgt_rule_line`] for the full widget API.

pub use zng_wgt_rule_line::{CollapseMode, RuleLine, collapse_scope};

/// Horizontal rule line widget and properties.
pub mod hr {
    pub use zng_wgt_rule_line::hr::{Hr, color, line_style, margin, stroke_thickness, width};
}

/// Vertical rule line widget and properties.
pub mod vr {
    pub use zng_wgt_rule_line::vr::{Vr, color, height, line_style, margin, stroke_thickness};
}
