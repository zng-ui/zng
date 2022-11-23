use std::{fmt, ops};

use derive_more as dm;

use crate::impl_from_and_into_var;

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

/// A length in bytes.
///
/// The value is stored in bytes, you can use associated functions to convert from other units or
/// you can use the [`ByteUnits`] extension methods to initialize from an integer literal.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    dm::Add,
    dm::AddAssign,
    dm::Sub,
    dm::SubAssign,
    dm::FromStr,
)]
pub struct ByteLength(pub usize);
impl_from_and_into_var! {
    fn from(bytes: usize) -> ByteLength {
        ByteLength(bytes)
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
    pub fn saturating_add(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_add(rhs.0))
    }

    /// Subtracts the two lengths without overflowing or wrapping.
    pub fn saturating_sub(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_sub(rhs.0))
    }

    /// Multiplies the two lengths without overflowing or wrapping.
    pub fn saturating_mul(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.saturating_mul(rhs.0))
    }

    // unstable
    ///// Divides the two lengths without overflowing or wrapping.
    //pub fn saturating_div(self, rhs: ByteLength) -> ByteLength {
    //    ByteLength(self.0.saturating_div(rhs.0))
    //}

    /// Adds the two lengths wrapping overflows.
    pub fn wrapping_add(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_add(rhs.0))
    }

    /// Subtracts the two lengths wrapping overflows.
    pub fn wrapping_sub(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_sub(rhs.0))
    }

    /// Multiplies the two lengths wrapping overflows.
    pub fn wrapping_mul(self, rhs: ByteLength) -> ByteLength {
        ByteLength(self.0.wrapping_mul(rhs.0))
    }

    /// Divides the two lengths wrapping overflows.
    pub fn wrapping_div(self, rhs: ByteLength) -> ByteLength {
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

    /// Divides the two lengths, returns `None` if the subtraction overflows.
    pub fn checked_div(self, rhs: ByteLength) -> Option<ByteLength> {
        self.0.checked_div(rhs.0).map(ByteLength)
    }
}

/// Constructors
impl ByteLength {
    /// From bytes.
    ///
    /// This is the same as `ByteLength(bytes)`.
    pub fn from_byte(bytes: usize) -> Self {
        ByteLength(bytes)
    }

    fn new(value: usize, scale: usize) -> Self {
        ByteLength(value.saturating_mul(scale))
    }

    /// From kibi-bytes.
    ///
    /// 1 kibi-byte equals 1024 bytes.
    pub fn from_kibi(kibi_bytes: usize) -> Self {
        Self::new(kibi_bytes, 1024)
    }

    /// From kilo-bytes.
    ///
    /// 1 kilo-byte equals 1000 bytes.
    pub fn from_kilo(kibi_bytes: usize) -> Self {
        Self::new(kibi_bytes, 1000)
    }

    /// From mebi-bytes.
    ///
    /// 1 mebi-byte equals 1024² bytes.
    pub fn from_mebi(mebi_bytes: usize) -> Self {
        Self::new(mebi_bytes, 1024usize.pow(2))
    }

    /// From mega-bytes.
    ///
    /// 1 mega-byte equals 1000² bytes.
    pub fn from_mega(mebi_bytes: usize) -> Self {
        Self::new(mebi_bytes, 1000usize.pow(2))
    }

    /// From gibi-bytes.
    ///
    /// 1 gibi-byte equals 1024³ bytes.
    pub fn from_gibi(gibi_bytes: usize) -> Self {
        Self::new(gibi_bytes, 1024usize.pow(3))
    }

    /// From giga-bytes.
    ///
    /// 1 giga-byte equals 1000³ bytes.
    pub fn from_giga(giba_bytes: usize) -> Self {
        Self::new(giba_bytes, 1000usize.pow(3))
    }

    /// From tebi-bytes.
    ///
    /// 1 tebi-byte equals 1024^4 bytes.
    pub fn from_tebi(gibi_bytes: usize) -> Self {
        Self::new(gibi_bytes, 1024usize.pow(4))
    }

    /// From tera-bytes.
    ///
    /// 1 tera-byte equals 1000^4 bytes.
    pub fn from_tera(giba_bytes: usize) -> Self {
        Self::new(giba_bytes, 1000usize.pow(4))
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
impl fmt::Display for ByteLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // alternate uses 0..=1000 units, normal used 0..=1024 units.

        if f.alternate() {
            if self.0 >= 1024usize.pow(4) {
                write!(f, "{:.2} tebibytes", self.tebis())
            } else if self.0 >= 1024usize.pow(3) {
                write!(f, "{:.2} gibibytes", self.gibis())
            } else if self.0 >= 1024usize.pow(2) {
                write!(f, "{:.2} mebibytes", self.mebis())
            } else if self.0 >= 1024 {
                write!(f, "{:.2} kibibytes", self.kibis())
            } else {
                write!(f, "{} bytes", self.bytes())
            }
        } else if self.0 >= 1000usize.pow(4) {
            write!(f, "{:.2} terabytes", self.teras())
        } else if self.0 >= 1000usize.pow(3) {
            write!(f, "{:.2} gigabytes", self.gigas())
        } else if self.0 >= 1000usize.pow(2) {
            write!(f, "{:.2} megabytes", self.megas())
        } else if self.0 >= 1000 {
            write!(f, "{:.2} kilobytes", self.kilos())
        } else {
            write!(f, "{} bytes", self.bytes())
        }
    }
}

impl<S: Into<Factor>> ops::Mul<S> for ByteLength {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self.0  = (self.0 as f64 * rhs.into().0 as f64) as usize;
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
        self.0  = (self.0 as f64 / rhs.into().0 as f64) as usize;
        self
    }
}
impl<S: Into<Factor>> ops::DivAssign<S> for ByteLength {
    fn div_assign(&mut self, rhs: S) {
        *self = *self / rhs;
    }
}