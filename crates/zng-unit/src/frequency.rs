use std::{fmt, ops};

use crate::Factor;

/// Extension methods for initializing [`Frequency`] values.
pub trait FrequencyUnits {
    /// Millihertz.
    ///
    /// See [`Frequency::from_millihertz`] for more details.
    fn millihertz(self) -> Frequency;

    /// Hertz.
    ///
    /// See [`Frequency::from_hertz`] for more details.
    fn hertz(self) -> Frequency;

    /// Megahertz.
    ///
    /// See [`Frequency::from_megahertz`] for more details.
    fn megahertz(self) -> Frequency;

    /// Gigahertz.
    ///
    /// See [`Frequency::from_gigahertz`] for more details.
    fn gigahertz(self) -> Frequency;

    /// Terahertz.
    ///
    /// See [`Frequency::from_terahertz`] for more details.
    fn terahertz(self) -> Frequency;
}
impl FrequencyUnits for u64 {
    fn millihertz(self) -> Frequency {
        Frequency::from_millihertz(self)
    }

    fn hertz(self) -> Frequency {
        Frequency::from_hertz(self as _)
    }

    fn megahertz(self) -> Frequency {
        Frequency::from_megahertz(self as _)
    }

    fn gigahertz(self) -> Frequency {
        Frequency::from_gigahertz(self as _)
    }

    fn terahertz(self) -> Frequency {
        Frequency::from_terahertz(self as _)
    }
}
impl FrequencyUnits for f64 {
    fn millihertz(self) -> Frequency {
        Frequency::from_millihertz(self.round() as _)
    }

    fn hertz(self) -> Frequency {
        Frequency::from_hertz(self)
    }

    fn megahertz(self) -> Frequency {
        Frequency::from_megahertz(self)
    }

    fn gigahertz(self) -> Frequency {
        Frequency::from_gigahertz(self)
    }

    fn terahertz(self) -> Frequency {
        Frequency::from_terahertz(self)
    }
}

/// A unit of frequency.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Frequency(u64);
impl ops::Add for Frequency {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for Frequency {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for Frequency {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for Frequency {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl ops::Mul<Factor> for Frequency {
    type Output = Frequency;

    fn mul(mut self, rhs: Factor) -> Self::Output {
        self.0 *= rhs;
        self
    }
}
impl ops::Div<Factor> for Frequency {
    type Output = Frequency;

    fn div(mut self, rhs: Factor) -> Self::Output {
        self.0 /= rhs;
        self
    }
}
impl ops::MulAssign<Factor> for Frequency {
    fn mul_assign(&mut self, rhs: Factor) {
        self.0 *= rhs;
    }
}
impl ops::DivAssign<Factor> for Frequency {
    fn div_assign(&mut self, rhs: Factor) {
        self.0 /= rhs;
    }
}
impl Frequency {
    /// Frequency in millihertz.
    pub const fn millihertz(&self) -> u64 {
        self.0
    }

    /// Frequency in hertz.
    pub const fn hertz(&self) -> f64 {
        self.0 as f64 / 1000.0
    }

    /// Frequency in kilohertz.
    pub const fn kilohertz(&self) -> f64 {
        self.hertz() / 1000.0
    }

    /// Frequency in megahertz.
    pub const fn megahertz(&self) -> f64 {
        self.kilohertz() / 1000.0
    }

    /// Frequency in gigahertz.
    pub const fn gigahertz(&self) -> f64 {
        self.megahertz() / 1000.0
    }

    /// Frequency in terahertz.
    pub const fn terahertz(&self) -> f64 {
        self.gigahertz() / 1000.0
    }

    /// Interval of one cycle at the frequency.
    pub const fn period(&self) -> std::time::Duration {
        if self.0 == 0 {
            return std::time::Duration::MAX;
        }
        let nanos = (1_000_000_000_000u128 / self.0 as u128) as u64;
        std::time::Duration::from_nanos(nanos)
    }
}
impl Frequency {
    /// From millihertz.
    pub const fn from_millihertz(z: u64) -> Self {
        Self(z)
    }

    /// From hertz.
    pub const fn from_hertz(z: f64) -> Self {
        Self::from_millihertz((z * 1000.0).round() as _)
    }

    /// From kilohertz.
    pub const fn from_kilohertz(z: f64) -> Self {
        Self::from_hertz(z * 1000.0)
    }

    /// From megahertz.
    pub const fn from_megahertz(z: f64) -> Self {
        Self::from_kilohertz(z * 1000.0)
    }

    /// From gigahertz.
    pub const fn from_gigahertz(z: f64) -> Self {
        Self::from_megahertz(z * 1000.0)
    }

    /// From terahertz.
    pub const fn from_terahertz(z: f64) -> Self {
        Self::from_gigahertz(z * 1000.0)
    }

    /// From interval of one cycle at the frequency.
    pub const fn from_period(p: std::time::Duration) -> Self {
        let nanos = p.as_nanos();
        if nanos == 0 {
            return Self(0);
        }
        let mhz = 1_000_000_000_000u128 / nanos;
        Self(mhz as u64)
    }
}
impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = self.0;
        if z >= 1_000_000_000_000_000 {
            write!(f, "{:.2}THz", self.terahertz())
        } else if z >= 1_000_000_000_000 {
            write!(f, "{:.2}GHz", self.gigahertz())
        } else if z >= 1_000_000_000 {
            write!(f, "{:.2}MHz", self.megahertz())
        } else if z >= 1_000_000 {
            write!(f, "{:.2}kHz", self.kilohertz())
        } else if z >= 1_000 {
            write!(f, "{:.2}Hz", self.hertz())
        } else {
            write!(f, "{z}mHz")
        }
    }
}
impl fmt::Debug for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Frequency").field(&self.0).finish()
        } else {
            write!(f, "Frequency({self})")
        }
    }
}
