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
//! use zng::{icon, prelude::*};
//! # fn example() {
//!
//! icon::ICONS.register(wgt_fn!(|a: icon::IconRequestArgs| {
//!     match a.name() {
//!         "accessibility" => Text!("A"),
//!         "settings" => Text!("S"),
//!         _ => UiNode::nil(),
//!     }
//! })); }
//! ```
//!
//! The example above registers a handler that provides two "icons" that are rendered by a `Text!` widgets.
//!
//! # Widget
//!
//! The [`Icon!`](struct@Icon) widget renders icons using an icon font, it allows setting the font and icon in a single value
//! and can auto size the font size, this makes it a better alternative to just using the `Text!` widget.
//!
//! ```
//! use zng::{icon, prelude::*};
//! # fn example() {
//!
//! # let _ =
//! icon::Icon! {
//!     ico = icon::material::rounded::req("accessibility");
//!     ico_size = 80;
//! }
//! # ; }
//! ```
//!
//! You can implement your own icon sets by providing [`GlyphIcon`] instances or a type that converts to `GlyphIcon`.
//! Glyph icons define a font name and a [`GlyphSource`] that can be a `char` or a ligature text.
//!
//! ```
//! # fn main() { }
//! use zng::{font, icon, prelude::*};
//! # async fn demo() {
//!
//! let font = font::CustomFont::from_file("Font Awesome 6 Free-Regular", r#"Font Awesome 6 Free-Regular-400.otf"#, 0);
//! font::FONTS.register(font).wait_rsp().await.unwrap();
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
//! See [`zng_wgt_text::icon`] for the full widget API.

pub use zng_wgt::{CommandIconExt, ICONS, IconRequestArgs};
pub use zng_wgt_text::icon::{GlyphIcon, GlyphSource, Icon, ico_color, ico_size};

/// Material Icons
///
/// The [Material Design Icons] can be embedded using the `"material_icons*"` crate features.
///
/// [Material Design Icons]: https://github.com/google/material-design-icons
///
/// ```toml
/// zng = { version = "0.22.8", features = ["material_icons"] }
/// ```
///
/// Handlers are registered for [`ICONS`] that provides the icons, the raw codepoints and glyph icon metadata is available in each font module.
///
/// If multiple material icons are enabled they are resolved in this order:
///
/// * outlined
/// * filled
/// * rounded
/// * sharp
///
/// You can disambiguate icons by using a the `"material/{set}/{name}"` where `{set}` is one of the items from the list above,
/// and `{name}` is the icon name.
///
/// # Subsetting
///
/// Compiling with the `"material_icons"` feature embeds the font file and a name table that around 2MB to the compiled executable.
/// Compiling with only one icon set adds around 350KB, you can optimize further by compiling with only the subset of icons used.
///
/// #### Step 1
///
/// To subset the icons you must create a profile file that lists all icons used by the app. To get started build this crate
/// with the `"material_icons_usage_recorder"` feature, run it and load every screen that uses an icon, close the app.
///
/// The subset profile is saved to `res/optimization-profiles/zng-wgt-material-icons.rec.subset` by default,
/// you can change the location by setting the `ZNG_MATERIAL_ICONS_PROFILE_FILE` env var.
///
/// The profile is a text file with format:
///
/// ```txt
/// # comments
///
/// {set}/{name}
/// ```
///
/// The generated file will has a comment header with instruction on how to manually add icons.
///
/// The profile file can be added to source control, the recorded entries are sorted so changes are stable.
///
/// #### Step 2
///
/// With the a subset profile ready you only need to enable the `"material_icons_subset"` crate feature. On build the profile
/// will be used to generate a subset font and only the used icons are added to the name table, greatly reducing
/// the binary size.
///
/// The profile file is found using the `ZNG_MATERIAL_ICONS_PROFILE_FILE` env var is set, or in the default location.
/// If two files `.rec.subset` and `.subset` are present with the same name both are used.
///
/// # Full API
///
/// See [`zng_wgt_material_icons`] for the full API.
pub mod material {
    #[cfg(feature = "material_icons_filled")]
    pub use zng_wgt_material_icons::filled;
    #[cfg(feature = "material_icons_outlined")]
    pub use zng_wgt_material_icons::outlined;
    #[cfg(feature = "material_icons_rounded")]
    pub use zng_wgt_material_icons::rounded;
    #[cfg(feature = "material_icons_sharp")]
    pub use zng_wgt_material_icons::sharp;
}
