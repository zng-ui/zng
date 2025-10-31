use std::fmt;

/// Measurement of pixels in a screen or points in print.
///
/// Internally the density is stored as pixels per centimeter.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct PxDensity(f32);
impl fmt::Debug for PxDensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PxDensity")
            .field(".ppi()", &self.ppi())
            .field(".ppcm()", &self.ppcm())
            .finish()
    }
}
/// `"{}"` writes `{:.0}ppi`, `"{:#}` writes `{:.0}ppcm`.
impl fmt::Display for PxDensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:.0}ppcm", self.ppcm())
        } else {
            write!(f, "{:.0}ppi", self.ppi())
        }
    }
}
impl Default for PxDensity {
    /// 96ppi.
    fn default() -> Self {
        PxDensity::new_ppi(96.0)
    }
}
impl PxDensity {
    /// Centimeters per inch.
    pub const CM_TO_INCH: f32 = 2.54;

    /// new from pixels per inch (PPI/DPI).
    pub fn new_ppi(pixels_per_inch: f32) -> Self {
        Self::new_ppcm(pixels_per_inch / Self::CM_TO_INCH)
    }

    /// New from pixels per meter (PPM/DPM).
    pub fn new_ppm(pixels_per_meter: f32) -> Self {
        Self::new_ppcm(pixels_per_meter / 100.0)
    }

    /// New from pixels per centimeter (PPCM/DPCM).
    pub fn new_ppcm(pixels_per_centimeter: f32) -> Self {
        Self(pixels_per_centimeter)
    }

    /// Density in pixels per inch (PPI/DPI).
    pub fn ppi(self) -> f32 {
        self.0 * Self::CM_TO_INCH
    }

    /// Density in pixels per meter (PPM/DPM).
    pub fn ppm(self) -> f32 {
        self.0 * 100.0
    }

    /// Density in pixels per centimeter (PPCM/DPCM).
    pub fn ppcm(self) -> f32 {
        self.0
    }
}
impl PartialEq for PxDensity {
    fn eq(&self, other: &Self) -> bool {
        super::about_eq(self.0, other.0, 0.001)
    }
}
impl Eq for PxDensity {}
impl std::hash::Hash for PxDensity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        super::about_eq_hash(self.0, 0.001, state)
    }
}
impl Ord for PxDensity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        super::about_eq_ord(self.0, other.0, 0.001)
    }
}
impl PartialOrd for PxDensity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Extension methods for initializing pixel density units.
///
/// # Examples
///
/// ```
/// # use zng_unit::*;
/// #
/// let p: PixelDensity = 96.ppi();
///
/// println!("ppi: {p}");
/// println!("ppcm: {p:#}");
/// ```
pub trait PxDensityUnits {
    /// Pixels-per-inch.
    fn ppi(self) -> PxDensity;
    /// Same as [`ppi`].
    ///
    /// [`ppi`]: PxDensityUnits::ppi
    fn dpi(self) -> PxDensity
    where
        Self: Sized,
    {
        self.ppi()
    }

    /// Pixels-per-centimeter.
    fn ppcm(self) -> PxDensity;
}
impl PxDensityUnits for u32 {
    fn ppi(self) -> PxDensity {
        PxDensity::new_ppi(self as f32)
    }

    fn ppcm(self) -> PxDensity {
        PxDensity::new_ppcm(self as f32)
    }
}
impl PxDensityUnits for f32 {
    fn ppi(self) -> PxDensity {
        PxDensity::new_ppi(self)
    }

    fn ppcm(self) -> PxDensity {
        PxDensity::new_ppcm(self)
    }
}

/// Pixel density value that can differ between dimensions.
pub type PxDensity2d = euclid::Size2D<PxDensity, PxDensity>;
