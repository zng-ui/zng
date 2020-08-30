use std::fmt;
use std::{
    f32::consts::*,
    ops::{Add, AddAssign},
};

#[derive(Debug, Copy, Clone)]
pub struct AngleRad(f32);
impl AngleRad {
    #[inline]
    pub fn new(rad: f32) -> Self {
        AngleRad(rad.rem_euclid(2.0 * PI))
    }

    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }
}
impl fmt::Display for AngleRad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rad", self.0)
    }
}

/// Angle in degrees.
#[derive(Debug, Copy, Clone)]
pub struct AngleDeg(f32);
impl AngleDeg {
    #[inline]
    pub fn new(deg: f32) -> Self {
        AngleDeg(deg.rem_euclid(360.0))
    }

    /// Degrees in `[0.0 ..= 360.0]`.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }
}
impl fmt::Display for AngleDeg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}º", self.0)
    }
}
impl PartialEq for AngleDeg {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0) < f32::EPSILON
    }
}
impl Eq for AngleDeg {}
impl Add for AngleDeg {
    type Output = AngleDeg;
    #[inline]
    fn add(self, rhs: AngleDeg) -> Self::Output {
        AngleDeg::new(self.0 + rhs.0)
    }
}
impl AddAssign for AngleDeg {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl From<AngleRad> for AngleDeg {
    fn from(rad: AngleRad) -> Self {
        AngleDeg::new(rad.0.to_degrees())
    }
}
pub trait AngleUnits {
    fn deg(self) -> AngleDeg;
    fn rad(self) -> AngleRad;
}

impl AngleUnits for f32 {
    #[inline]
    fn deg(self) -> AngleDeg {
        AngleDeg(self)
    }

    #[inline]
    fn rad(self) -> AngleRad {
        AngleRad(self)
    }
}
