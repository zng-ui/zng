//! Named primary, secondary and tertiary colors.
//!
//! You can use [`darken`] and [`lighten`] to derive more shades from these colors.
//!
//! [`darken`]: Rgba::darken
//! [`lighten`]: Rgba::lighten

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

/// <div style="display: inline-block; background-color:#000000; width:20px; height:20px;"></div> Black, <code>#000000</code>, <code>rgb(0, 0, 0)</code>.
pub const BLACK: Rgba = rgb!(0, 0, 0);

/// <div style="display: inline-block; background-color:#808080; width:20px; height:20px;"></div> Gray, <code>#808080</code>, <code>rgb(128, 128, 128)</code>.
pub const GRAY: Rgba = rgb!(128, 128, 128);

/// <div style="display: inline-block; background-color:#FFFFFF; width:20px; height:20px;"></div> White, <code>#FFFFFF</code>, <code>rgb(255, 255, 255)</code>.
pub const WHITE: Rgba = rgb!(255, 255, 255);

/// <div style="display: inline-block; background-color:#FF0000; width:20px; height:20px;"></div> Red, <code>#FF0000</code>, <code>rgb(255, 0, 0)</code>.
pub const RED: Rgba = rgb!(255, 0, 0);

/// <div style="display: inline-block; background-color:#FF8000; width:20px; height:20px;"></div> Orange, <code>#FF8000</code>, <code>rgb(255, 128, 0)</code>.
pub const ORANGE: Rgba = rgb!(255, 128, 0);

/// <div style="display: inline-block; background-color:#FFFF00; width:20px; height:20px;"></div> Yellow, <code>#FFFF00</code>, <code>rgb(255, 255, 0)</code>.
pub const YELLOW: Rgba = rgb!(255, 255, 0);

/// <div style="display: inline-block; background-color:#80FF00; width:20px; height:20px;"></div> Lime, <code>#80FF00</code>, <code>rgb(128, 255, 0)</code>.
pub const LIME: Rgba = rgb!(128, 255, 0);

/// <div style="display: inline-block; background-color:#00FF00; width:20px; height:20px;"></div> Green, <code>#00FF00</code>, <code>rgb(0, 255, 0)</code>.
pub const GREEN: Rgba = rgb!(0, 255, 0);

/// <div style="display: inline-block; background-color:#00FF80; width:20px; height:20px;"></div> Spring, <code>#00FF80</code>, <code>rgb(0, 255, 128)</code>.
pub const SPRING: Rgba = rgb!(0, 255, 128);

/// <div style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></div> Cyan, <code>#00FFFF</code>, <code>rgb(0, 255, 255)</code>.
pub const CYAN: Rgba = rgb!(0, 255, 255);

/// <div style="display: inline-block; background-color:#0080FF; width:20px; height:20px;"></div> Azure, <code>#0080FF</code>, <code>rgb(0, 128, 255)</code>.
pub const AZURE: Rgba = rgb!(0, 128, 255);

/// <div style="display: inline-block; background-color:#0000FF; width:20px; height:20px;"></div> Blue, <code>#0000FF</code>, <code>rgb(0, 0, 255)</code>.
pub const BLUE: Rgba = rgb!(0, 0, 255);

/// <div style="display: inline-block; background-color:#8000FF; width:20px; height:20px;"></div> Violet, <code>#8000FF</code>, <code>rgb(128, 0, 255)</code>.
pub const VIOLET: Rgba = rgb!(128, 0, 255);

/// <div style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></div> Magenta, <code>#FF00FF</code>, <code>rgb(255, 0, 255)</code>.
pub const MAGENTA: Rgba = rgb!(255, 0, 255);

/// <div style="display: inline-block; background-color:#FF0080; width:20px; height:20px;"></div> Rose, <code>#FF0080</code>, <code>rgb(255, 0, 128)</code>.
pub const ROSE: Rgba = rgb!(255, 0, 128);
