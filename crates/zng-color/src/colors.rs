//! Named primary, secondary and tertiary colors.
//!
//! You can use [`darken`] and [`lighten`] to derive more shades from these colors.
//!
//! [`darken`]: crate::MixAdjust::darken
//! [`lighten`]: crate::MixAdjust::lighten

use zng_var::context_var;

use crate::{LightDark, light_dark};

use super::Rgba;

macro_rules! rgb {
    ($r:literal, $g:literal, $b:literal) => {
        Rgba {
            red: $r as f32 / 255.,
            green: $g as f32 / 255.,
            blue: $b as f32 / 255.,
            alpha: 1.0,
        }
    };
}

/// <span style="display: inline-block; background-color:#000000; width:20px; height:20px;"></span> Black, `#000000`, `rgb(0, 0, 0)`.
pub const BLACK: Rgba = rgb!(0, 0, 0);

/// <span style="display: inline-block; background-color:#808080; width:20px; height:20px;"></span> Gray, `#808080`, `rgb(128, 128, 128)`.
pub const GRAY: Rgba = rgb!(128, 128, 128);

/// <span style="display: inline-block; background-color:#FFFFFF; width:20px; height:20px;"></span> White, `#FFFFFF`, `rgb(255, 255, 255)`.
pub const WHITE: Rgba = rgb!(255, 255, 255);

/// <span style="display: inline-block; background-color:#FF0000; width:20px; height:20px;"></span> Red, `#FF0000`, `rgb(255, 0, 0)`.
pub const RED: Rgba = rgb!(255, 0, 0);

/// <span style="display: inline-block; background-color:#FF8000; width:20px; height:20px;"></span> Orange, `#FF8000`, `rgb(255, 128, 0)`.
pub const ORANGE: Rgba = rgb!(255, 128, 0);

/// <span style="display: inline-block; background-color:#FFFF00; width:20px; height:20px;"></span> Yellow, `#FFFF00`, `rgb(255, 255, 0)`.
pub const YELLOW: Rgba = rgb!(255, 255, 0);

/// <span style="display: inline-block; background-color:#80FF00; width:20px; height:20px;"></span> Lime, `#80FF00`, `rgb(128, 255, 0)`.
pub const LIME: Rgba = rgb!(128, 255, 0);

/// <span style="display: inline-block; background-color:#00FF00; width:20px; height:20px;"></span> Green, `#00FF00`, `rgb(0, 255, 0)`.
pub const GREEN: Rgba = rgb!(0, 255, 0);

/// <span style="display: inline-block; background-color:#00FF80; width:20px; height:20px;"></span> Spring, `#00FF80`, `rgb(0, 255, 128)`.
pub const SPRING: Rgba = rgb!(0, 255, 128);

/// <span style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></span> Cyan, `#00FFFF`, `rgb(0, 255, 255)`.
pub const CYAN: Rgba = rgb!(0, 255, 255);

/// <span style="display: inline-block; background-color:#0080FF; width:20px; height:20px;"></span> Azure, `#0080FF`, `rgb(0, 128, 255)`.
pub const AZURE: Rgba = rgb!(0, 128, 255);

/// <span style="display: inline-block; background-color:#0000FF; width:20px; height:20px;"></span> Blue, `#0000FF`, `rgb(0, 0, 255)`.
pub const BLUE: Rgba = rgb!(0, 0, 255);

/// <span style="display: inline-block; background-color:#8000FF; width:20px; height:20px;"></span> Violet, `#8000FF`, `rgb(128, 0, 255)`.
pub const VIOLET: Rgba = rgb!(128, 0, 255);

/// <span style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></span> Magenta, `#FF00FF`, `rgb(255, 0, 255)`.
pub const MAGENTA: Rgba = rgb!(255, 0, 255);

/// <span style="display: inline-block; background-color:#FF0080; width:20px; height:20px;"></span> Rose, `#FF0080`, `rgb(255, 0, 128)`.
pub const ROSE: Rgba = rgb!(255, 0, 128);

context_var! {
    /// Color that contrasts with the text color.
    pub static ACCENT_COLOR_VAR: LightDark = BLUE;

    /// Seed color for widget background.
    ///
    /// See also [`LightDarkVarExt`] for helper methods implemented on [`LightDark`] variables.
    ///
    /// [`LightDarkVarExt`]: crate::LightDarkVarExt
    pub static BASE_COLOR_VAR: LightDark = light_dark(WHITE, BLACK);
}
