//! Icon widget and types.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_text::icon`] for the full widget API.

pub use zero_ui_wgt_text::icon::{ico_color, ico_size, CommandIconExt, GlyphIcon, GlyphSource, Icon};

#[cfg(feature = "material_icons")]
pub use zero_ui_wgt_material_icons::{filled, outlined, rounded, sharp, MaterialFonts, MaterialIcon};
