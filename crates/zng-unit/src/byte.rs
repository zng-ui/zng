use std::{fmt, ops};

use super::Factor;

/// Extension methods for initializing [`ByteLength`] values.
pub trait ByteUnits {
    /// Bytes.
    ///
    /// See [`ByteLength`] for more details.
    fn bytes(self) -> ByteLength;
    /// Kibi-bytes.
    ///
    /// See [`ByteLength::from_kibi`] for more details.
    fn kibibytes(self) -> ByteLength;
    /// Kilo-bytes.
    ///
    /// See [`ByteLength::from_kilo`] for more details.
    fn kilobytes(self) -> ByteLength;

    /// Mebi-bytes.
    ///
    /// See [`ByteLength::from_mebi`] for more details.
    fn mebibytes(self) -> ByteLength;
    /// Mega-bytes.
    ///
    /// See [`ByteLength::from_mega`] for more details.
    fn megabytes(self) -> ByteLength;

    /// Gibi-bytes.
    ///
    /// See [`ByteLength::from_gibi`] for more details.
    fn gibibytes(self) -> ByteLength;
    /// Giga-bytes.
    ///
    /// See [`ByteLength::from_giga`] for more details.
    fn gigabytes(self) -> ByteLength;

    /// Tebi-bytes.
    ///
    /// See [`ByteLength::from_tebi`] for more details.
    fn tebibytes(self) -> ByteLength;
    /// Tera-bytes.
    ///
    /// See [`ByteLength::from_tera`] for more details.
    fn terabytes(self) -> ByteLength;
}
impl ByteUnits for usize {
    fn bytes(self) -> ByteLength {
        ByteLength(self)
    }

    fn kibibytes(self) -> ByteLength {
        ByteLength::from_kibi(self)
    }

    fn kilobytes(self) -> ByteLength {
        ByteLength::from_kilo(self)
    }

    fn mebibytes(self) -> ByteLength {
        ByteLength::from_mebi(self)
    }

    fn megabytes(self) -> ByteLength {
        ByteLength::from_mega(self)
    }

    fn gibibytes(self) -> ByteLength {
        ByteLength::from_gibi(self)
    }

    fn gigabytes(self) -> ByteLength {
        ByteLength::from_giga(self)
    }

    fn tebibytes(self) -> ByteLength {
        ByteLength::from_tebi(self)
    }

    fn terabytes(self) -> ByteLength {
        ByteLength::from_tera(self)
    }
}
impl ByteUnits for f64 {
    fn bytes(self) -> ByteLength {
        ByteLength::from_byte_f64(self)
    }

    fn kibibytes(self) -> ByteLength {
        ByteLength::from_kibi_f64(self)
    }

    fn kilobytes(self) -> ByteLength {
        ByteLength::from_kilo_f64(self)
    }

    fn mebibytes(self) -> ByteLength {
        ByteLength::from_mebi_f64(self)
    }

    fn megabytes(self) -> ByteLength {
        ByteLength::from_mega_f64(self)
    }

    fn gibibytes(self) -> ByteLength {
        ByteLength::from_gibi_f64(self)
    }

    fn gigabytes(self) -> ByteLength {
        ByteLength::from_giga_f64(self)
    }

    fn tebibytes(self) -> ByteLength {
        ByteLength::from_tebi_f64(self)
    }

    fn terabytes(self) -> ByteLength {
        ByteLength::from_tera_f64(self)
    }
}

/// A length in bytes.
///
/// The value is stored in bytes, you can use associated functions to convert from other units or
/// you can use the [`ByteUnits`] extension methods to initialize from an integer literal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ByteLength(pub usize);
impl From<usize> for ByteLength {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
impl ops::Add for ByteLength {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for ByteLength {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for ByteLength {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for ByteLength {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ByteLength {
    /// Length in bytes.
    ///
    /// This is the same as `.0`.
    pub fn bytes(&self) -> usize {
        self.0
    }

    fn scaled(self, scale: f64) -> f64 {
        self.0 as f64 / scale
    }

    /// Length in kibi-bytes.
    pub fn kibis(self) -> f64 {
        self.scaled(1024.0)
    }

    /// Length in kilo-bytes.
    pub fn kilos(self) -> f64 {
        self.scaled(1000.0)
    }

    /// Length in mebi-bytes.
    pub fn mebis(self) -> f64 {
        self.scaled(1024.0f64.powi(2))
    }

    /// Length in mega-bytes.
    pub fn megas(self) -> f64 {
        self.scaled(1000.0f64.powi(2))
    }

    /// Length in gibi-bytes.
    pub fn gibis(self) -> f64 {
        self.scaled(1024.0f64.powi(3))
    }

    /// Length in giga-bytes.
    pub fn gigas(self) -> f64 {
        self.scaled(1000.0f64.powi(3))
    }

    /// Length in tebi-bytes.
    pub fn tebis(self) -> f64 {
        self.scaled(1024.0f64.powi(4))
    }

    /// Length in tera-bytes.
    pub fn teras(self) -> f64 {
        self.scaled(1000.0f64.powi(4))
    }

    /// Maximum representable byte length.
    pub const MAX: ByteLength = ByteLength(usize::MAX);

    /// Adds the two lengths without overflowing or wrapping.
    pub const fn saturating_add(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_add(rhs.0))
    }

    /// Subtracts the two lengths without overflowing or wrapping.
    pub const fn saturating_sub(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_sub(rhs.0))
    }

    /// Multiplies the two lengths without overflowing or wrapping.
    pub const fn saturating_mul(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_mul(rhs.0))
    }

    // unstable
    ///// Divides the two lengths without overflowing or wrapping.
    //pub fn saturating_div(self, rhs: ByteLength) -> ByteLength {
    //    ByteLength(self.0.saturating_div(rhs.0))
    //}

    /// Adds the two lengths wrapping overflows.
    pub const fn wrapping_add(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_add(rhs.0))
    }

    /// Subtracts the two lengths wrapping overflows.
    pub const fn wrapping_sub(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_sub(rhs.0))
    }

    /// Multiplies the two lengths wrapping overflows.
    pub const fn wrapping_mul(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_mul(rhs.0))
    }

    /// Divides the two lengths wrapping overflows.
    pub const fn wrapping_div(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_div(rhs.0))
    }

    /// Adds the two lengths, returns `None` if the sum overflows.
    pub fn checked_add(self, rhs: ByteLength) -> Option<ByteLength> {
        self.0.checked_add(rhs.0).map(ByteLength)
    }

    /// Subtracts the two lengths, returns `None` if the subtraction overflows.
    pub fn checked_sub(self, rhs: ByteLength) -> Option<ByteLength> {
        self.0.checked_sub(rhs.0).map(ByteLength)
    }

    /// Multiplies the two lengths, returns `None` if the sum overflows.
    pub fn checked_mul(self, rhs: ByteLength) -> Option<ByteLength> {
        self.0.checked_mul(rhs.0).map(ByteLength)
    }

    /// Divides the two lengths, returns `None` if the division overflows.
    pub fn checked_div(self, rhs: ByteLength) -> Option<ByteLength> {
        self.0.checked_div(rhs.0).map(ByteLength)
    }
}

/// Constructors
impl ByteLength {
    /// From bytes.
    ///
    /// This is the same as `ByteLength(bytes)`.
    pub const fn from_byte(bytes: usize) -> Self {
        ByteLength(bytes)
    }

    /// From fractional bytes.
    ///
    /// Just rounds the value.
    pub const fn from_byte_f64(bytes: f64) -> Self {
        ByteLength(bytes.round() as _)
    }

    const fn new(value: usize, scale: usize) -> Self {
        ByteLength(value.saturating_mul(scale))
    }

    const fn new_f64(value: f64, scale: f64) -> Self {
        ByteLength::from_byte_f64(value * scale)
    }

    /// From kibi-bytes.
    ///
    /// 1 kibi-byte equals 1024 bytes.
    pub const fn from_kibi(kibi_bytes: usize) -> Self {
        Self::new(kibi_bytes, 1024)
    }
    /// From kibi-bytes.
    ///
    /// 1 kibi-byte equals 1024 bytes.
    pub const fn from_kibi_f64(kibi_bytes: f64) -> Self {
        Self::new_f64(kibi_bytes, 1024.0)
    }

    /// From kilo-bytes.
    ///
    /// 1 kilo-byte equals 1000 bytes.
    pub const fn from_kilo(kilo_bytes: usize) -> Self {
        Self::new(kilo_bytes, 1000)
    }
    /// From kilo-bytes.
    ///
    /// 1 kilo-byte equals 1000 bytes.
    pub const fn from_kilo_f64(kilo_bytes: f64) -> Self {
        Self::new_f64(kilo_bytes, 1000.0)
    }

    /// From mebi-bytes.
    ///
    /// 1 mebi-byte equals 1024² bytes.
    pub const fn from_mebi(mebi_bytes: usize) -> Self {
        Self::new(mebi_bytes, 1024usize.pow(2))
    }
    /// From mebi-bytes.
    ///
    /// 1 mebi-byte equals 1024² bytes.
    pub const fn from_mebi_f64(mebi_bytes: f64) -> Self {
        Self::new_f64(mebi_bytes, 1024.0 * 1024.0)
    }

    /// From mega-bytes.
    ///
    /// 1 mega-byte equals 1000² bytes.
    pub const fn from_mega(mega_bytes: usize) -> Self {
        Self::new(mega_bytes, 1000usize.pow(2))
    }
    /// From mega-bytes.
    ///
    /// 1 mega-byte equals 1000² bytes.
    pub const fn from_mega_f64(mebi_bytes: f64) -> Self {
        Self::new_f64(mebi_bytes, 1000.0 * 1000.0)
    }

    /// From gibi-bytes.
    ///
    /// 1 gibi-byte equals 1024³ bytes.
    pub const fn from_gibi(gibi_bytes: usize) -> Self {
        Self::new(gibi_bytes, 1024usize.pow(3))
    }
    /// From gibi-bytes.
    ///
    /// 1 gibi-byte equals 1024³ bytes.
    pub const fn from_gibi_f64(gibi_bytes: f64) -> Self {
        Self::new_f64(gibi_bytes, 1024.0 * 1024.0 * 1024.0)
    }

    /// From giga-bytes.
    ///
    /// 1 giga-byte equals 1000³ bytes.
    pub const fn from_giga(giga_bytes: usize) -> Self {
        Self::new(giga_bytes, 1000usize.pow(3))
    }
    /// From giga-bytes.
    ///
    /// 1 giga-byte equals 1000³ bytes.
    pub const fn from_giga_f64(giga_bytes: f64) -> Self {
        Self::new_f64(giga_bytes, 1000.0 * 1000.0 * 1000.0)
    }

    /// From tebi-bytes.
    ///
    /// 1 tebi-byte equals 1024^4 bytes.
    pub const fn from_tebi(tebi_bytes: usize) -> Self {
        Self::new(tebi_bytes, 1024usize.pow(4))
    }
    /// From tebi-bytes.
    ///
    /// 1 tebi-byte equals 1024^4 bytes.
    pub const fn from_tebi_f64(tebi_bytes: f64) -> Self {
        Self::new_f64(tebi_bytes, 1024.0 * 1024.0 * 1024.0 * 1024.0)
    }

    /// From tera-bytes.
    ///
    /// 1 tera-byte equals 1000^4 bytes.
    pub const fn from_tera(tera_bytes: usize) -> Self {
        Self::new(tera_bytes, 1000usize.pow(4))
    }
    /// From tera-bytes.
    ///
    /// 1 tera-byte equals 1000^4 bytes.
    pub const fn from_tera_f64(tera_bytes: f64) -> Self {
        Self::new_f64(tera_bytes, 1000.0 * 1000.0 * 1000.0 * 1000.0)
    }
}

impl ByteLength {
    /// Compares and returns the maximum of two lengths.
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Compares and returns the minimum of two lengths.
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl fmt::Debug for ByteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("ByteLength").field(&self.0).finish()
        } else {
            write!(f, "ByteLength({self})")
        }
    }
}

/// Alternative mode prints in binary units (kibi, mebi, gibi, tebi)
impl fmt::Display for ByteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            if self.0 >= 1024usize.pow(4) {
                write!(f, "{:.2}TiB", self.tebis())
            } else if self.0 >= 1024usize.pow(3) {
                write!(f, "{:.2}GiB", self.gibis())
            } else if self.0 >= 1024usize.pow(2) {
                write!(f, "{:.2}MiB", self.mebis())
            } else if self.0 >= 1024 {
                write!(f, "{:.2}KiB", self.kibis())
            } else {
                write!(f, "{}B", self.bytes())
            }
        } else if self.0 >= 1000usize.pow(4) {
            write!(f, "{:.2}TB", self.teras())
        } else if self.0 >= 1000usize.pow(3) {
            write!(f, "{:.2}GB", self.gigas())
        } else if self.0 >= 1000usize.pow(2) {
            write!(f, "{:.2}MB", self.megas())
        } else if self.0 >= 1000 {
            write!(f, "{:.2}kB", self.kilos())
        } else {
            write!(f, "{}B", self.bytes())
        }
    }
}
/// Parses `"##"`, `"##TiB"`, `"##GiB"`, `"##MiB"`, `"##KiB"`, `"##B"`, `"##TB"`, `"##GB"`, `"##MB"`, `"##kB"` and `"##B"` where `##` is an `usize`.
impl std::str::FromStr for ByteLength {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(n) = s.strip_suffix("TiB") {
            n.parse().map(ByteLength::from_tebi_f64)
        } else if let Some(n) = s.strip_suffix("GiB") {
            n.parse().map(ByteLength::from_gibi_f64)
        } else if let Some(n) = s.strip_suffix("MiB") {
            n.parse().map(ByteLength::from_mebi_f64)
        } else if let Some(n) = s.strip_suffix("KiB") {
            n.parse().map(ByteLength::from_kibi_f64)
        } else if let Some(n) = s.strip_suffix("TB") {
            n.parse().map(ByteLength::from_tera_f64)
        } else if let Some(n) = s.strip_suffix("GB") {
            n.parse().map(ByteLength::from_giga_f64)
        } else if let Some(n) = s.strip_suffix("MB") {
            n.parse().map(ByteLength::from_mega_f64)
        } else if let Some(n) = s.strip_suffix("kB") {
            n.parse().map(ByteLength::from_kilo_f64)
        } else if let Some(n) = s.strip_suffix("B") {
            n.parse().map(ByteLength::from_byte_f64)
        } else {
            s.parse().map(ByteLength::from_byte_f64)
        }
    }
}
impl<S: Into<Factor>> ops::Mul<S> for ByteLength {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self.0 = (self.0 as f64 * rhs.into().0 as f64) as usize;
        self
    }
}
impl<S: Into<Factor>> ops::MulAssign<S> for ByteLength {
    fn mul_assign(&mut self, rhs: S) {
        *self = *self * rhs;
    }
}
impl<S: Into<Factor>> ops::Div<S> for ByteLength {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self.0 = (self.0 as f64 / rhs.into().0 as f64) as usize;
        self
    }
}
impl<S: Into<Factor>> ops::DivAssign<S> for ByteLength {
    fn div_assign(&mut self, rhs: S) {
        *self = *self / rhs;
    }
}
