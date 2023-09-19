//! System config types.

use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};

use crate::units::{Dip, DipSize};

/// System settings needed for implementing double/triple clicks.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
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
pub struct KeyRepeatConfig {
    /// Delay before repeat starts.
    pub start_delay: Duration,
    /// Delay before each repeat event after the first.
    pub interval: Duration,
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
pub struct AnimationsConfig {
    /// If animation are enabled.
    ///
    /// People with photo-sensitive epilepsy usually disable animations system wide.
    pub enabled: bool,

    /// Interval of the caret blink animation.
    pub caret_blink_interval: Duration,
    /// Duration after which the blink animation stops.
    pub caret_blink_timeout: Duration,
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
pub struct LocaleConfig {
    /// BCP-47 language tags, if the locale can be obtained.
    pub langs: Vec<String>,
}

/// Text anti-aliasing.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontAntiAliasing {
    /// Uses the operating system configuration.
    Default,
    /// Sub-pixel anti-aliasing if a fast implementation is available, otherwise uses `Alpha`.
    Subpixel,
    /// Alpha blending anti-aliasing.
    Alpha,
    /// Disable anti-aliasing.
    Mono,
}
impl Default for FontAntiAliasing {
    fn default() -> Self {
        Self::Default
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorScheme {
    /// Dark foreground, light background.
    Light,

    /// Light foreground, dark background.
    Dark,
}
impl Default for ColorScheme {
    /// Light.
    fn default() -> Self {
        ColorScheme::Light
    }
}
