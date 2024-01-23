//! Rule line widgets and properties.
//!
//! A rule line is a horizontal or vertical separator line, this module provides 3 widgets the [`RuleLine!`](struct@RuleLine)
//! base that can dynamically change orientation and the [`hr::Hr!`](struct@hr::Hr) and [`vr::Vr!`](struct@vr::Vr) that represents
//! each orientation and can be styled separately.
//!
//! ```
//! use zero_ui::prelude::*;
//! # fn demo() {
//!
//! # let _ =
//! Window! {
//!     context_menu = ContextMenu!(ui_vec![
//!         Button!(zero_ui::app::NEW_CMD.scoped(WINDOW.id())),
//!         Button!(zero_ui::app::OPEN_CMD.scoped(WINDOW.id())),
//!         Hr!(),
//!         Button!(zero_ui::app::EXIT_CMD),
//!     ]);
//! }
//! # ; }
//! ```
//!
//! The example above uses the `Hr!` widget in a context menu to separate the commands into two groups.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_rule_line`] for the full widget API.

pub use zero_ui_wgt_rule_line::RuleLine;

/// Horizontal rule line widget and properties.
pub mod hr {
    pub use zero_ui_wgt_rule_line::hr::{color, line_style, margin, stroke_thickness, Hr};
}

/// Vertical rule line widget and properties.
pub mod vr {
    pub use zero_ui_wgt_rule_line::vr::{color, line_style, margin, stroke_thickness, Vr};
}
