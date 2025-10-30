//! System config types.

use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};

use zng_txt::Txt;
use zng_unit::{Dip, DipSize, Rgba};
use zng_var::impl_from_and_into_var;

/// System settings needed for implementing double/triple clicks.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct MultiClickConfig {
    /// Maximum time interval between clicks.
    ///
    /// Only repeated clicks within this time interval can count as double-clicks.
    pub time: Duration,

    /// Maximum (x, y) distance in pixels.
    ///
    /// Only repeated clicks that are within this distance of the first click can count as double-clicks.
    pub area: DipSize,
}
impl MultiClickConfig {
    /// New config.
    pub fn new(time: Duration, area: DipSize) -> Self {
        Self { time, area }
    }
}
impl Default for MultiClickConfig {
    /// `500ms` and `4, 4`.
    fn default() -> Self {
        Self {
            time: Duration::from_millis(500),
            area: DipSize::splat(Dip::new(4)),
        }
    }
}

/// System settings needed to implementing touch gestures.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct TouchConfig {
    /// Maximum (x, y) distance between a touch start and end that generates a touch click.
    ///
    /// Area can be disregarded if the touch is not ambiguous. This usually defines the initial lag
    /// for a single finger drag gesture.
    pub tap_area: DipSize,

    /// Maximum (x, y) distance that a subsequent touch click is linked with the previous one as a double click.
    ///
    /// Area can be disregarded if the touch is not ambiguous.
    pub double_tap_area: DipSize,

    /// Maximum time between start and end in the `tap_area` that generates a touch click.
    ///
    /// Time can be disregarded if the touch is not ambiguous. This usually defines the *long press* delay.
    pub tap_max_time: Duration,

    /// Maximum time between taps that generates a double click.
    pub double_tap_max_time: Duration,

    /// Minimum velocity that can be considered a fling gesture, in dip per seconds.
    pub min_fling_velocity: Dip,

    /// Fling velocity ceiling, in dip per seconds.
    pub max_fling_velocity: Dip,
}

impl TouchConfig {
    /// New config.
    pub fn new(
        tap_area: DipSize,
        double_tap_area: DipSize,
        tap_max_time: Duration,
        double_tap_max_time: Duration,
        min_fling_velocity: Dip,
        max_fling_velocity: Dip,
    ) -> Self {
        Self {
            tap_area,
            double_tap_area,
            tap_max_time,
            double_tap_max_time,
            min_fling_velocity,
            max_fling_velocity,
        }
    }
}
impl Default for TouchConfig {
    fn default() -> Self {
        Self {
            tap_area: DipSize::splat(Dip::new(8)),
            double_tap_area: DipSize::splat(Dip::new(28)),
            tap_max_time: Duration::from_millis(500),
            double_tap_max_time: Duration::from_millis(500),
            min_fling_velocity: Dip::new(50),
            max_fling_velocity: Dip::new(8000),
        }
    }
}

/// System settings that define the key pressed repeat.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct KeyRepeatConfig {
    /// Delay before repeat starts.
    pub start_delay: Duration,
    /// Delay before each repeat event after the first.
    pub interval: Duration,
}
impl KeyRepeatConfig {
    /// New config.
    pub fn new(start_delay: Duration, interval: Duration) -> Self {
        Self { start_delay, interval }
    }
}
impl Default for KeyRepeatConfig {
    /// 600ms, 100ms.
    fn default() -> Self {
        Self {
            start_delay: Duration::from_millis(600),
            interval: Duration::from_millis(100),
        }
    }
}

/// System settings that control animations.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct AnimationsConfig {
    /// If animation are enabled.
    ///
    /// People with photo-sensitive epilepsy usually disable animations system wide.
    pub enabled: bool,

    /// Interval of the caret blink animation.
    ///
    /// This is the duration the cursor stays visible.
    pub caret_blink_interval: Duration,
    /// Duration after which the blink animation stops.
    pub caret_blink_timeout: Duration,
}
impl AnimationsConfig {
    /// New config.
    pub fn new(enabled: bool, caret_blink_interval: Duration, caret_blink_timeout: Duration) -> Self {
        Self {
            enabled,
            caret_blink_interval,
            caret_blink_timeout,
        }
    }
}
impl Default for AnimationsConfig {
    /// true, 530ms, 5s.
    fn default() -> Self {
        Self {
            enabled: true,
            caret_blink_interval: Duration::from_millis(530),
            caret_blink_timeout: Duration::from_secs(5),
        }
    }
}

/// System settings that define the locale.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize, Default)]
#[non_exhaustive]
pub struct LocaleConfig {
    /// BCP-47 language tags, if the locale can be obtained.
    pub langs: Vec<Txt>,
}
impl LocaleConfig {
    /// New config.
    pub fn new(langs: Vec<Txt>) -> Self {
        Self { langs }
    }
}

/// Text anti-aliasing.
#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[cfg_attr(feature = "var", zng_var::impl_property_value)]
pub enum FontAntiAliasing {
    /// Uses the operating system configuration.
    #[default]
    Default,
    /// Sub-pixel anti-aliasing if a fast implementation is available, otherwise uses `Alpha`.
    Subpixel,
    /// Alpha blending anti-aliasing.
    Alpha,
    /// Disable anti-aliasing.
    Mono,
}
impl fmt::Debug for FontAntiAliasing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontAntiAliasing::")?;
        }
        match self {
            FontAntiAliasing::Default => write!(f, "Default"),
            FontAntiAliasing::Subpixel => write!(f, "Subpixel"),
            FontAntiAliasing::Alpha => write!(f, "Alpha"),
            FontAntiAliasing::Mono => write!(f, "Mono"),
        }
    }
}

/// Color scheme preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
#[cfg_attr(feature = "var", zng_var::impl_property_value)]
pub enum ColorScheme {
    /// Dark text, light background.
    #[default]
    Light,

    /// Light text, dark background.
    Dark,
}
impl_from_and_into_var! {
    fn from(_: ColorScheme) -> Option<ColorScheme>;
}

/// System colors and color scheme.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ColorsConfig {
    /// Color scheme (light/dark) preference.
    pub scheme: ColorScheme,
    /// Accent color.
    ///
    /// Accent color preference.
    ///
    /// Expect a saturated color that contrasts with the text color.
    pub accent: Rgba,
}
impl ColorsConfig {
    /// New config.
    pub fn new(scheme: ColorScheme, accent: Rgba) -> Self {
        Self { scheme, accent }
    }
}
impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            scheme: Default::default(),
            accent: Rgba::new(10, 10, 200, 255),
        }
    }
}

/// Window chrome (decorations) preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChromeConfig {
    /// Window manager prefers that the window renders a custom chrome.
    ///
    /// This is also called "Client-Side Decorations", it is `true` in GNOME+Wayland.
    pub prefer_custom: bool,

    /// If the Window manager provides a chrome.
    ///
    /// When this is `false` the view-process implementation may provide just a very basic fallback chrome,
    /// if the app-process still requests system chrome.
    pub provided: bool,
}
impl ChromeConfig {
    /// New config.
    pub fn new(prefer_custom: bool, provided: bool) -> Self {
        Self { prefer_custom, provided }
    }

    /// If system prefers custom and does not provide chrome.
    ///
    /// Note that a chromeless window is not forbidden if this is `true`.
    pub fn needs_custom(&self) -> bool {
        self.prefer_custom && !self.provided
    }
}
impl Default for ChromeConfig {
    /// Prefer custom false, provided true.
    fn default() -> Self {
        Self {
            prefer_custom: false,
            provided: true,
        }
    }
}
