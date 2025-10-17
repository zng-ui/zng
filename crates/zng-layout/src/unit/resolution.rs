use std::fmt;

use zng_var::{animation::Transitionable, impl_from_and_into_var};

/// Pixels-per-inch resolution.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Transitionable)]
#[serde(transparent)]
pub struct Ppi(pub f32);
impl Default for Ppi {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0)
    }
}
impl PartialEq for Ppi {
    fn eq(&self, other: &Self) -> bool {
        super::about_eq(self.0, other.0, 0.0001)
    }
}
impl Eq for Ppi {}
impl std::hash::Hash for Ppi {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        super::about_eq_hash(self.0, 0.0001, state)
    }
}
impl Ord for Ppi {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        super::about_eq_ord(self.0, other.0, 0.0001)
    }
}
impl PartialOrd for Ppi {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Pixels-per-meter resolution.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Transitionable)]
#[serde(transparent)]
pub struct Ppm(pub f32);
impl PartialEq for Ppm {
    fn eq(&self, other: &Self) -> bool {
        super::about_eq(self.0, other.0, 0.0001)
    }
}
impl Eq for Ppm {}
impl std::hash::Hash for Ppm {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        super::about_eq_hash(self.0, 0.0001, state)
    }
}
impl Ord for Ppm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        super::about_eq_ord(self.0, other.0, 0.0001)
    }
}
impl PartialOrd for Ppm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Default for Ppm {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0).into()
    }
}

/// Extension methods for initializing resolution units.
///
/// # Examples
///
/// ```
/// use zng_layout::unit::*;
///
/// let ppm: Ppm = 96.dpi().into();
/// ```
pub trait ResolutionUnits {
    /// Pixels-per-inch.
    fn ppi(self) -> Ppi;
    /// Same as [`ppi`].
    ///
    /// [`ppi`]: ResolutionUnits::ppi
    fn dpi(self) -> Ppi
    where
        Self: Sized,
    {
        self.ppi()
    }

    /// Pixels-per-meter.
    fn ppm(self) -> Ppm;
}
impl ResolutionUnits for u32 {
    fn ppi(self) -> Ppi {
        Ppi(self as f32)
    }

    fn ppm(self) -> Ppm {
        Ppm(self as f32)
    }
}
impl ResolutionUnits for f32 {
    fn ppi(self) -> Ppi {
        Ppi(self)
    }

    fn ppm(self) -> Ppm {
        Ppm(self)
    }
}

/// Prints `"{}ppi"`. Alternate prints `"{}dpi"`.
impl fmt::Display for Ppi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}dpi", self.0)
        } else {
            write!(f, "{}ppi", self.0)
        }
    }
}
impl fmt::Display for Ppm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ppm", self.0)
    }
}
impl std::str::FromStr for Ppi {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_suffix("ppi").or_else(|| s.strip_suffix("dpi")).unwrap_or(s);
        Ok(Ppi(s.parse()?))
    }
}
impl std::str::FromStr for Ppm {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_suffix("ppm").unwrap_or(s);
        Ok(Ppm(s.parse()?))
    }
}

impl_from_and_into_var! {
    fn from(ppi: Ppi) -> Ppm {
        Ppm(ppi.0 * 39.3701)
    }

    fn from(ppm: Ppm) -> Ppi {
        Ppi(ppm.0 / 39.3701)
    }
}

// TODO(breaking) integrate this with ImagePpi
