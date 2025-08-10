#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Material icons for the [`Icon!`] widget.
//!
//! A map from name to icon codepoint is defined in a module for each font. The font files are embedded
//! by default and can are registered using the [`MaterialIconsManager`] app extension. The extension
//! also registers [`ICONS`] handlers that provide the icons.
//!
//! The icons are from the [Material Design Icons] project.
//!
//! [`Icon!`]: struct@zng_wgt_text::icon::Icon
//! [`ICONS`]: struct@zng_wgt::ICONS
//! [Material Design Icons]: https://github.com/google/material-design-icons
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

/// Material icon fonts manager.
///
/// This app extension registers the fonts in `"embedded"` builds and registers [`ICONS`] handlers that provide the icons.
///
/// [`ICONS`]: struct@zng_wgt::ICONS
#[derive(Default)]
#[non_exhaustive]
pub struct MaterialIconsManager;
impl MaterialIconsManager {
    #[cfg(all(
        feature = "embedded",
        any(feature = "outlined", feature = "filled", feature = "rounded", feature = "sharp")
    ))]
    fn register_fonts(&self) {
        let sets = [
            #[cfg(feature = "outlined")]
            (outlined::FONT_NAME, outlined::FONT_BYTES),
            #[cfg(feature = "filled")]
            (filled::FONT_NAME, filled::FONT_BYTES),
            #[cfg(feature = "rounded")]
            (rounded::FONT_NAME, rounded::FONT_BYTES),
            #[cfg(feature = "sharp")]
            (sharp::FONT_NAME, sharp::FONT_BYTES),
        ];

        for (name, bytes) in sets {
            let font = zng_ext_font::CustomFont::from_bytes(name, zng_ext_font::FontDataRef::from_static(bytes), 0);
            zng_ext_font::FONTS.register(font);
        }
    }
}
#[cfg(feature = "embedded")]
impl zng_app::AppExtension for MaterialIconsManager {
    #[cfg(any(feature = "outlined", feature = "filled", feature = "rounded", feature = "sharp"))]
    fn init(&mut self) {
        use zng_wgt::{ICONS, IconRequestArgs, prelude::UiNode, wgt_fn};
        use zng_wgt_text::icon::{GlyphIcon, Icon};

        self.register_fonts();

        ICONS.register(wgt_fn!(|args: IconRequestArgs| {
            if let Some(strong_key) = args.name().strip_prefix("material/") {
                #[expect(clippy::type_complexity)]
                let sets: &[(&str, fn(&str) -> Option<GlyphIcon>)] = &[
                    #[cfg(feature = "outlined")]
                    ("outlined/", outlined::get),
                    #[cfg(feature = "filled")]
                    ("filled/", filled::get),
                    #[cfg(feature = "rounded")]
                    ("rounded/", rounded::get),
                    #[cfg(feature = "sharp")]
                    ("sharp/", sharp::get),
                ];
                for (name, get) in sets {
                    if let Some(key) = strong_key.strip_prefix(name)
                        && let Some(ico) = get(key)
                    {
                        return Icon!(ico);
                    }
                }
            }

            UiNode::nil()
        }));

        ICONS.register_fallback(wgt_fn!(|args: IconRequestArgs| {
            let sets = [
                #[cfg(feature = "outlined")]
                outlined::get,
                #[cfg(feature = "filled")]
                filled::get,
                #[cfg(feature = "rounded")]
                rounded::get,
                #[cfg(feature = "sharp")]
                sharp::get,
            ];
            for get in sets {
                if let Some(ico) = get(args.name()) {
                    return Icon!(ico);
                }
            }
            UiNode::nil()
        }));
    }
}

#[cfg(any(feature = "outlined", feature = "filled", feature = "rounded", feature = "sharp"))]
macro_rules! getters {
    ($FONT_NAME:ident, $MAP:ident) => {
        /// Gets the [`GlyphIcon`].
        pub fn get(key: &str) -> Option<GlyphIcon> {
            Some(GlyphIcon::new($FONT_NAME.clone(), *$MAP.get(key)?))
        }

        /// Require the [`GlyphIcon`], logs an error if not found.
        ///
        /// # Panics
        ///
        /// Panics if the `key` is not found.
        ///
        /// [`GlyphIcon`]: struct@zng_wgt_text::icon::GlyphIcon
        pub fn req(key: &str) -> GlyphIcon {
            match get(key) {
                Some(g) => g,
                None => {
                    tracing::error!("icon {key:?} not found in `outlined`");
                    GlyphIcon::new("", '\0')
                }
            }
        }

        /// All icons.
        pub fn all() -> impl ExactSizeIterator<Item = (&'static str, GlyphIcon)> {
            $MAP.entries()
                .map(|(key, val)| (*key, GlyphIcon::new($FONT_NAME.clone(), *val)))
        }
    };
}

/// Outline icons.
///  
/// This is the "Material Icons Outlined" font.
///
/// # Icons
///
/// Use the [`ICONS`] service with key `"material/outlined/{name}"` or `"{name}"` to get an widget that renders the icon.
///
/// Use [`outlined::req`] to get a [`GlyphIcon`] directly for use in the [`Icon!`] widget.
///
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
/// [`GlyphIcon`]: struct@zng_wgt_text::icon::GlyphIcon
/// [`ICONS`]: struct@zng_wgt::ICONS
///
/// | Name | Icon |
/// |------|------|
#[doc = include_str!(concat!(env!("OUT_DIR"), "/generated.outlined.docs.txt"))]
#[cfg(feature = "outlined")]
pub mod outlined {
    use zng_ext_font::FontName;
    use zng_wgt_text::icon::GlyphIcon;

    /// "Material Icons Outlined".
    pub const FONT_NAME: FontName = FontName::from_static("Material Icons Outlined");

    /// Embedded font bytes.
    #[cfg(feature = "embedded")]
    pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsOutlined-Regular.otf");

    include!(concat!(env!("OUT_DIR"), "/generated.outlined.map.rs"));
    getters!(FONT_NAME, MAP);
}

/// Filled icons.
///
/// This is the "Material Icons" font.
///
/// # Icons
///
/// Use the [`ICONS`] service with key `"material/filled/{name}"` or `"{name}"` to get an widget that renders the icon.
///
/// Use [`filled::req`] to get a [`GlyphIcon`] directly for use in the [`Icon!`] widget.
///
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
/// [`GlyphIcon`]: struct@zng_wgt_text::icon::GlyphIcon
/// [`ICONS`]: struct@zng_wgt::ICONS
///
/// | Name | Icon |
/// |------|------|
#[doc = include_str!(concat!(env!("OUT_DIR"), "/generated.filled.docs.txt"))]
#[cfg(feature = "filled")]
pub mod filled {
    use zng_ext_font::FontName;
    use zng_wgt_text::icon::GlyphIcon;

    /// "Material Icons".
    pub const FONT_NAME: FontName = FontName::from_static("Material Icons");

    /// Embedded font bytes.
    #[cfg(feature = "embedded")]
    pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIcons-Regular.ttf");

    include!(concat!(env!("OUT_DIR"), "/generated.filled.map.rs"));
    getters!(FONT_NAME, MAP);
}

/// Rounded icons.
///  
/// This is the "Material Icons Rounded" font.
///
/// # Icons
///
/// Use the [`ICONS`] service with key `"material/rounded/{name}"` or `"{name}"` to get an widget that renders the icon.
///
/// Use [`rounded::req`] to get a [`GlyphIcon`] directly for use in the [`Icon!`] widget.
///
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
/// [`GlyphIcon`]: struct@zng_wgt_text::icon::GlyphIcon
/// [`ICONS`]: struct@zng_wgt::ICONS
///
/// | Name | Icon |
/// |------|------|
#[doc = include_str!(concat!(env!("OUT_DIR"), "/generated.rounded.docs.txt"))]
#[cfg(feature = "rounded")]
pub mod rounded {
    use zng_ext_font::FontName;
    use zng_wgt_text::icon::GlyphIcon;

    /// "Material Icons Rounded".
    pub const FONT_NAME: FontName = FontName::from_static("Material Icons Rounded");

    /// Embedded font bytes.
    #[cfg(feature = "embedded")]
    pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsRound-Regular.otf");

    include!(concat!(env!("OUT_DIR"), "/generated.rounded.map.rs"));
    getters!(FONT_NAME, MAP);
}

/// Sharp icons.
///  
/// This is the "Material Icons Sharp" font.
///
/// # Icons
///
/// Use the [`ICONS`] service with key `"material/sharp/{name}"` or `"{name}"` to get an widget that renders the icon.
///
/// Use [`sharp::req`] to get a [`GlyphIcon`] directly for use in the [`Icon!`] widget.
///
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
/// [`GlyphIcon`]: struct@zng_wgt_text::icon::GlyphIcon
/// [`ICONS`]: struct@zng_wgt::ICONS
///
/// | Name | Icon |
/// |------|------|
#[doc = include_str!(concat!(env!("OUT_DIR"), "/generated.sharp.docs.txt"))]
#[cfg(feature = "sharp")]
pub mod sharp {
    use zng_ext_font::FontName;
    use zng_wgt_text::icon::GlyphIcon;

    /// "Material Icons Sharp".
    pub const FONT_NAME: FontName = FontName::from_static("Material Icons Sharp");

    /// Embedded font bytes.
    #[cfg(feature = "embedded")]
    pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsSharp-Regular.otf");

    include!(concat!(env!("OUT_DIR"), "/generated.sharp.map.rs"));
    getters!(FONT_NAME, MAP);
}
