//! Font types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use zng_task::channel::IpcBytes;
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
#[non_exhaustive]
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
impl FontOptions {
    /// New font options.
    pub fn new(aa: FontAntiAliasing, synthetic_bold: bool, synthetic_oblique: bool) -> Self {
        Self {
            aa,
            synthetic_bold,
            synthetic_oblique,
        }
    }
}

/// Extra font options send with text glyphs.
pub type GlyphOptions = FontOptions;

/// Font feature name, `*b"hlig"` for example.
pub type FontVariationName = [u8; 4];

/// Glyph index with position.
#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[non_exhaustive]
pub struct GlyphInstance {
    /// Glyph id.
    pub index: GlyphIndex,
    /// Glyph position.
    pub point: euclid::Point2D<f32, Px>,
}
impl GlyphInstance {
    /// New glyph.
    pub fn new(index: GlyphIndex, point: euclid::Point2D<f32, Px>) -> Self {
        Self { index, point }
    }
}

/// Glyph index in a font.
pub type GlyphIndex = u32;

/// Represents font face data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum IpcFontBytes {
    /// Custom font bytes.
    Bytes(IpcBytes),
    /// Font file path in a restricted system fonts directory.
    ///
    /// The path must be safe for potential memory mapping. If the file is
    /// as restricted as the current executable if can be considered safe.
    System(PathBuf),
}
