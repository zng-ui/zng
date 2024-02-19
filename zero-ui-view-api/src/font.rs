//! Font types.

use serde::{Deserialize, Serialize};
use zero_ui_unit::{AngleDegree, AngleUnits as _};

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
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FontOptions {
    /// Font render mode.
    ///
    /// Default value must be already resolved here, it falls-back to Subpixel.
    pub aa: FontAntiAliasing,

    /// If synthetic bold is enabled.
    pub synthetic_bold: bool,
    /// Skew angle, 0º is disabled.
    pub synthetic_italics: AngleDegree,
}

impl Default for FontOptions {
    fn default() -> Self {
        Self {
            aa: FontAntiAliasing::default(),
            synthetic_bold: false,
            synthetic_italics: 0.deg(),
        }
    }
}

/// Font feature name, `*b"hlig"` for example.
pub type FontVariationName = [u8; 4];
