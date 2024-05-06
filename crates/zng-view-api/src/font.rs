//! Font types.

use serde::{Deserialize, Serialize};
use zng_unit::Px;

use crate::{config::FontAntiAliasing, declare_id};

declare_id! {
    /// Font resource in a renderer cache.
    ///
    /// The View Process defines the ID.
    pub struct FontFaceId(_);

    /// Sized font in a renderer.
    ///
    /// The View Process defines the ID.
    pub struct FontId(_);
}

/// Extra font options.
#[derive(Default, Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FontOptions {
    /// Font render mode.
    ///
    /// Default value must be already resolved here, it falls back to Subpixel.
    pub aa: FontAntiAliasing,

    /// If synthetic bold is enabled.
    pub synthetic_bold: bool,
    /// If synthetic skew is enabled.
    pub synthetic_oblique: bool,
}

/// Extra font options send with text glyphs.
pub type GlyphOptions = FontOptions;

/// Font feature name, `*b"hlig"` for example.
pub type FontVariationName = [u8; 4];

/// Glyph index with position.
#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GlyphInstance {
    #[allow(missing_docs)]
    pub index: GlyphIndex,
    #[allow(missing_docs)]
    pub point: euclid::Point2D<f32, Px>,
}

/// Glyph index in a font.
pub type GlyphIndex = u32;
