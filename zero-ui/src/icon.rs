//! Icon widget and types.
//!
//! Icons for this widget are defined in a text font. The [Material Design Icons]
//! can be embedded using the crate feature `"material_icons"`.
//!
//! [Material Design Icons]: https://github.com/google/material-design-icons
//!
//! ```toml
//! zero-ui = { version =  "*", features = ["material_icons"] }
//! ```
//!
//! ```
//! use zero_ui::{prelude::*, icon};
//! # let _ = APP.defaults();
//!
//! # let _ =
//! icon::Icon! {
//!     ico = icon::material_rounded::ACCESSIBILITY;
//!     ico_size = 80;
//! }
//! # ;
//! ```
//!
//! You can implement your own icon sets by providing [`GlyphIcon`] instances or a type that converts to `GlyphIcon`, the
//! [`MaterialIcon`] type is an example of this. Glyph icons define a font name and a [`GlyphSource`] that can be a `char`
//! or a ligature text.
//! 
//! ```
//! # fn main() { }
//! use zero_ui::{prelude::*, icon};
//! # fn demo() {
//! # let _ = APP.defaults();
//! 
//! let font = CustomFont::from_file(
//!     "Font Awesome 6 Free-Regular",
//!     r#"Font Awesome 6 Free-Regular-400.otf"#,
//!     0,
//! );
//! FONTS.register(font).wait_into_rsp().await.unwrap();
//! 
//! # let _ =
//! icon::Icon! {
//!     ico = icon::GlyphIcon::new("Font Awesome 6 Free-Regular", "address-book").with_ligatures();
//!     ico_size = 80;
//! }
//! # ;
//! # }
//! ```
//!
//! The example above loads an icon font and display one of the icons selected using a ligature that matches `"address-book"`.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_text::icon`] for the full widget API.

pub use zero_ui_wgt_text::icon::{ico_color, ico_size, CommandIconExt, GlyphIcon, GlyphSource, Icon};

#[cfg(feature = "zero-ui-wgt-material-icons")]
pub use zero_ui_wgt_material_icons::{MaterialFonts, MaterialIcon};

#[cfg(feature = "material_icons_filled")]
pub use zero_ui_wgt_material_icons::filled as material_filled;
#[cfg(feature = "material_icons_outlined")]
pub use zero_ui_wgt_material_icons::outlined as material_outlined;
#[cfg(feature = "material_icons_rounded")]
pub use zero_ui_wgt_material_icons::rounded as material_rounded;
#[cfg(feature = "material_icons_sharp")]
pub use zero_ui_wgt_material_icons::sharp as material_sharp;
