//! Font types.

use serde::{Deserialize, Serialize};
use zero_ui_unit::Px;

use crate::{config::FontAntiAliasing, declare_id, unit::PxToWr};

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
    /// Default value must be already resolved here, it falls-back to Subpixel.
    pub aa: FontAntiAliasing,

    /// If synthetic bold is enabled.
    pub synthetic_bold: bool,
    /// If synthetic skew is enabled.
    pub synthetic_oblique: bool,
}

impl PxToWr for FontOptions {
    type AsDevice = Option<webrender_api::FontInstanceOptions>;

    type AsLayout = Option<webrender_api::FontInstanceOptions>;

    type AsWorld = Option<webrender_api::GlyphOptions>;

    fn to_wr_device(self) -> Self::AsDevice {
        self.to_wr()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        self.to_wr().map(|o| webrender_api::GlyphOptions {
            render_mode: o.render_mode,
            flags: o.flags,
        })
    }

    fn to_wr(self) -> Self::AsLayout {
        if self == FontOptions::default() {
            None
        } else {
            Some(webrender_api::FontInstanceOptions {
                render_mode: match self.aa {
                    FontAntiAliasing::Default => webrender_api::FontRenderMode::Subpixel,
                    FontAntiAliasing::Subpixel => webrender_api::FontRenderMode::Subpixel,
                    FontAntiAliasing::Alpha => webrender_api::FontRenderMode::Alpha,
                    FontAntiAliasing::Mono => webrender_api::FontRenderMode::Mono,
                },
                flags: if self.synthetic_bold {
                    webrender_api::FontInstanceFlags::SYNTHETIC_BOLD
                } else {
                    webrender_api::FontInstanceFlags::empty()
                },
                synthetic_italics: webrender_api::SyntheticItalics::from_degrees(if self.synthetic_oblique { 14.0 } else { 0.0 }),
            })
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
pub struct GlyphInstance {
    ///
    pub index: GlyphIndex,
    ///
    pub point: euclid::Point2D<f32, Px>,
}

/// Glyph index in a font.
pub type GlyphIndex = u32;

pub(crate) fn cast_glyphs_to_wr(glyphs: &[GlyphInstance]) -> &[webrender_api::GlyphInstance] {
    debug_assert_eq!(
        std::mem::size_of::<GlyphInstance>(),
        std::mem::size_of::<webrender_api::GlyphInstance>()
    );
    debug_assert_eq!(std::mem::size_of::<GlyphIndex>(), std::mem::size_of::<webrender_api::GlyphIndex>());
    debug_assert_eq!(
        std::mem::size_of::<euclid::Point2D<f32, Px>>(),
        std::mem::size_of::<webrender_api::units::LayoutPoint>()
    );

    // SAFETY: GlyphInstance is a copy of the webrender_api
    unsafe { std::mem::transmute(glyphs) }
}
