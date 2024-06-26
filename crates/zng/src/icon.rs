//! Icons service, icon font widget and other types.
//!
//! # Service
//!
//! The [`ICONS`] service bridges icon providers and icon users. Icon theme providers can register
//! handlers that provide a node that renders the icon identified by name. Widget styles or other UI
//! only need to request the icon, avoiding having to embed icon resources in lib crates and avoiding
//! icons having a fixed appearance.
//!
//! ```
//! use zng::{prelude::*, icon, widget::node::NilUiNode};
//! # let _scope = APP.defaults();
//!
//! icon::ICONS.register(wgt_fn!(|a: icon::IconRequestArgs| {
//!     match a.name() {
//!         "accessibility" => icon::Icon!(icon::material_rounded::ACCESSIBILITY).boxed(),
//!         "settings" => icon::Icon!(icon::material_rounded::SETTINGS).boxed(),
//!         _ => NilUiNode.boxed()
//!     }
//! }));
//! ```
//!
//! The example above registers a handler that provides two icons.
//!
//! # Widget
//!
//! The [`Icon!`](struct@Icon) widget renders icons using an icon font.
//!
//! ```
//! use zng::{prelude::*, icon};
//! # let _scope = APP.defaults();
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
//! use zng::{prelude::*, icon, font};
//! # async fn demo() {
//! # let _scope = APP.defaults();
//!
//! let font = font::CustomFont::from_file(
//!     "Font Awesome 6 Free-Regular",
//!     r#"Font Awesome 6 Free-Regular-400.otf"#,
//!     0,
//! );
//! font::FONTS.register(font).wait_into_rsp().await.unwrap();
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
//! # Material Icons
//!
//! The [Material Design Icons] can be embedded using the crate feature `"material_icons"`.
//!
//! [Material Design Icons]: https://github.com/google/material-design-icons
//!
//! ```toml
//! zng = { version = "0.9.1", features = ["material_icons"] }
//! ```
//! Note that if `"material_icons_outlined"` feature is enabled the default `APP` will register an [`ICONS`] handler that provides
//! many of the icons needed by the Zng widgets and apps in general.
//!
//! # Full API
//!
//! See [`zng_wgt_text::icon`] for the full widget API.

pub use zng_wgt_text::icon::{ico_color, ico_size, GlyphIcon, GlyphSource, Icon};

#[cfg(any(
    feature = "material_icons_filled",
    feature = "material_icons_outlined",
    feature = "material_icons_rounded",
    feature = "material_icons_sharp",
))]
pub use zng_wgt_material_icons::{MaterialFonts, MaterialIcon};

#[cfg(feature = "material_icons_filled")]
pub use zng_wgt_material_icons::filled as material_filled;
#[cfg(feature = "material_icons_outlined")]
pub use zng_wgt_material_icons::outlined as material_outlined;
#[cfg(feature = "material_icons_rounded")]
pub use zng_wgt_material_icons::rounded as material_rounded;
#[cfg(feature = "material_icons_sharp")]
pub use zng_wgt_material_icons::sharp as material_sharp;

pub use zng_wgt::{CommandIconExt, IconRequestArgs, ICONS};
