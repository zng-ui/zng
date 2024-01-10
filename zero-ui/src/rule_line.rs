//! Rule line widgets and properties.
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
