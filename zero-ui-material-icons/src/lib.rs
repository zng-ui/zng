//! Material icons for the [`icon!`] widget.
//!
//! The constants are defined in a module for each font. The font files are embedded
//! and can be registered using the [`MaterialFonts`] app extension.
//!
//! The icons are from the [Material Design Icons] project.
//!
//! [`icon!`]: mod@zero_ui::widgets::icon
//! [Material Design Icons]: https://github.com/google/material-design-icons

#![warn(missing_docs)]
#![cfg_attr(doc_nightly, feature(doc_cfg))]

use std::fmt;

use zero_ui::{
    core::{
        app::AppExtension,
        impl_from_and_into_var,
        text::{CustomFont, FontDataRef, FontName, Fonts, FontsExt},
    },
    widgets::icon::GlyphIcon,
};

/// Material fonts.
///
/// You can call the [`MaterialFonts::register`] method yourself before creating any windows or you can
/// use this struct as an [`AppExtension`] that does the same thing on app init.
#[cfg(feature = "embedded")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
pub struct MaterialFonts;
#[cfg(feature = "embedded")]
impl MaterialFonts {
    /// Register the material fonts.
    pub fn register(fonts: &mut Fonts) {
        let sets = [
            (outlined::meta::FONT_NAME, outlined::meta::FONT_BYTES),
            (filled::meta::FONT_NAME, filled::meta::FONT_BYTES),
            (rounded::meta::FONT_NAME, rounded::meta::FONT_BYTES),
            (sharp::meta::FONT_NAME, sharp::meta::FONT_BYTES),
            (two_tone::meta::FONT_NAME, two_tone::meta::FONT_BYTES),
        ];

        for (name, bytes) in sets {
            let font = CustomFont::from_bytes(name, FontDataRef::from_static(bytes), 0);
            fonts.register(font).unwrap();
        }
    }
}
#[cfg(feature = "embedded")]
impl AppExtension for MaterialFonts {
    fn init(&mut self, ctx: &mut zero_ui::prelude::AppContext) {
        Self::register(ctx.services.fonts())
    }
}

/// Represents a material font icon.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MaterialIcon {
    /// Font name.
    pub font: FontName,
    /// Constant name of the icon.
    pub name: &'static str,
    /// Codepoint.
    pub code: char,
}
impl_from_and_into_var! {
    fn from(icon: MaterialIcon) -> GlyphIcon {
        GlyphIcon {
            font: icon.font,
            glyph: icon.code.into(),
        }
    }
}
impl fmt::Display for MaterialIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Outline icons.
///  
/// This is the "Material Icons Outlined" font.
#[cfg(feature = "outlined")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "outlined")))]
pub mod outlined {
    use super::*;

    /// Font metadata.
    pub mod meta {
        use super::*;

        /// "Material Icons Outlined".
        pub const FONT_NAME: FontName = FontName::from_static("Material Icons Outlined");

        /// Embedded font bytes.
        #[cfg(feature = "embedded")]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
        pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsOutlined-Regular.otf");
    }

    include!(concat!(env!("OUT_DIR"), "/generated.outlined.rs"));
}

/// Filled icons.
///
/// This is the "Material Icons" font.
#[cfg(feature = "filled")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "filled")))]
pub mod filled {
    use super::*;

    /// Font metadata.
    pub mod meta {
        use super::*;

        /// "Material Icons".
        pub const FONT_NAME: FontName = FontName::from_static("Material Icons");

        /// Embedded font bytes.
        #[cfg(feature = "embedded")]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
        pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIcons-Regular.ttf");
    }

    include!(concat!(env!("OUT_DIR"), "/generated.filled.rs"));
}

/// Rounded icons.
///  
/// This is the "Material Icons Rounded" font.
#[cfg(feature = "rounded")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "rounded")))]
pub mod rounded {
    use super::*;

    /// Font metadata.
    pub mod meta {
        use super::*;

        /// "Material Icons Rounded".
        pub const FONT_NAME: FontName = FontName::from_static("Material Icons Rounded");

        /// Embedded font bytes.
        #[cfg(feature = "embedded")]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
        pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsRound-Regular.otf");
    }

    include!(concat!(env!("OUT_DIR"), "/generated.rounded.rs"));
}

/// Sharp icons.
///  
/// This is the "Material Icons Sharp" font.
#[cfg(feature = "sharp")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "sharp")))]
pub mod sharp {
    use super::*;

    /// Font metadata.
    pub mod meta {
        use super::*;

        /// "Material Icons Sharp".
        pub const FONT_NAME: FontName = FontName::from_static("Material Icons Sharp");

        /// Embedded font bytes.
        #[cfg(feature = "embedded")]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
        pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsSharp-Regular.otf");
    }

    include!(concat!(env!("OUT_DIR"), "/generated.sharp.rs"));
}

/// Sharp icons.
///  
/// This is the "Material Icons Two-Tone" font.
#[cfg(feature = "two_tone")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "two_tone")))]
pub mod two_tone {
    use super::*;

    /// Font metadata.
    pub mod meta {
        use super::*;

        /// "Material Icons Two-Tone".
        pub const FONT_NAME: FontName = FontName::from_static("Material Icons Two-Tone");

        /// Embedded font bytes.
        #[cfg(feature = "embedded")]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "embedded")))]
        pub const FONT_BYTES: &[u8] = include_bytes!("../fonts/MaterialIconsTwoTone-Regular.otf");
    }

    include!(concat!(env!("OUT_DIR"), "/generated.two_tone.rs"));
}
