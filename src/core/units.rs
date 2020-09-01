use derive_more as dm;
use std::f32::consts::*;

const TAU: f32 = 2.0 * PI;

/// Angle in radians.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} rad", self.0)]
pub struct AngleRad(f32);
impl AngleRad {
    #[inline]
    pub fn modulo(self) -> f32 {
        self.0.rem_euclid(TAU)
    }

    #[inline]
    pub fn about_eq(&self, other: Self) -> bool {
        (self.0 - other.0) < f32::EPSILON
    }
}
impl From<AngleGrad> for AngleRad {
    fn from(grad: AngleGrad) -> Self {
        AngleRad(grad.0 * PI / 200.0)
    }
}
impl From<AngleDeg> for AngleRad {
    fn from(deg: AngleDeg) -> Self {
        AngleRad(deg.0.to_radians())
    }
}
impl From<AngleTurn> for AngleRad {
    fn from(turn: AngleTurn) -> Self {
        AngleRad(turn.0 * TAU)
    }
}

/// Angle in gradians.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} gon", self.0)]
pub struct AngleGrad(pub f32);
impl AngleGrad {
    #[inline]
    pub fn modulo(self) -> f32 {
        self.0.rem_euclid(400.0)
    }

    #[inline]
    pub fn about_eq(&self, other: Self) -> bool {
        (self.0 - other.0) < f32::EPSILON
    }
}
impl From<AngleRad> for AngleGrad {
    fn from(rad: AngleRad) -> Self {
        AngleGrad(rad.0 * 200.0 / PI)
    }
}
impl From<AngleDeg> for AngleGrad {
    fn from(deg: AngleDeg) -> Self {
        AngleGrad(deg.0 * 10.0 / 9.0)
    }
}
impl From<AngleTurn> for AngleGrad {
    fn from(turn: AngleTurn) -> Self {
        AngleGrad(turn.0 * 400.0)
    }
}

/// Angle in degrees.
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{}º", self.0)]
pub struct AngleDeg(pub f32);
impl AngleDeg {
    /// Degrees in `[0.0 ..= 360.0]`.
    #[inline]
    pub fn modulo(self) -> f32 {
        self.0.rem_euclid(360.0)
    }

    #[inline]
    pub fn about_eq(&self, other: Self) -> bool {
        (self.0 - other.0) < f32::EPSILON
    }
}
impl From<AngleRad> for AngleDeg {
    fn from(rad: AngleRad) -> Self {
        AngleDeg(rad.0.to_degrees())
    }
}
impl From<AngleGrad> for AngleDeg {
    fn from(grad: AngleGrad) -> Self {
        AngleDeg(grad.0 * 9.0 / 10.0)
    }
}
impl From<AngleTurn> for AngleDeg {
    fn from(turn: AngleTurn) -> Self {
        AngleDeg(turn.0 * 360.0)
    }
}

/// Angle in turns (complete rotations)
#[derive(Debug, dm::Display, Copy, Clone, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign, PartialEq)]
#[display(fmt = "{} tr", self.0)]
pub struct AngleTurn(pub f32);
impl AngleTurn {
    #[inline]
    pub fn modulo(self) -> f32 {
        self.0.rem_euclid(1.0)
    }

    #[inline]
    pub fn about_eq(&self, other: Self) -> bool {
        (self.0 - other.0) < f32::EPSILON
    }
}
impl From<AngleRad> for AngleTurn {
    fn from(rad: AngleRad) -> Self {
        AngleTurn(rad.0 / TAU)
    }
}
impl From<AngleGrad> for AngleTurn {
    fn from(grad: AngleGrad) -> Self {
        AngleTurn(grad.0 / 400.0)
    }
}
impl From<AngleDeg> for AngleTurn {
    fn from(deg: AngleDeg) -> Self {
        AngleTurn(deg.0 / 360.0)
    }
}

pub trait AngleUnits {
    fn rad(self) -> AngleRad;
    fn grad(self) -> AngleGrad;
    fn deg(self) -> AngleDeg;
    fn turn(self) -> AngleTurn;
}

impl AngleUnits for f32 {
    #[inline]
    fn rad(self) -> AngleRad {
        AngleRad(self)
    }

    #[inline]
    fn grad(self) -> AngleGrad {
        AngleGrad(self)
    }

    #[inline]
    fn deg(self) -> AngleDeg {
        AngleDeg(self)
    }

    #[inline]
    fn turn(self) -> AngleTurn {
        AngleTurn(self)
    }
}
