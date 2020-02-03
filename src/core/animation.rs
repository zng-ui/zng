use crate::core::context::Vars;
use crate::core::var::VarValue;
use std::ops::Mul;
use std::time::Duration;

/// A [variable](crate::core::var::Var) animation.
pub trait VarAnimation<T: VarValue> {
    /// Current animating value.
    fn get_step(&self, vars: &Vars) -> &T;

    /// Update animating value.
    fn update_step(&self, vars: &Vars) -> Option<&T>;

    /// Borrow end value.
    fn get_end(&self) -> &T;

    /// Stops the animation and returns  the end value.
    fn end(self) -> T;
}

/// Easing function value. Mostly between 0.0 and 1.0 but can over/undershot.
#[derive(Debug, Clone, Copy)]
pub struct EasingStep(pub f32);

impl EasingStep {
    /// Gets the scale value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }
}

/// Easing function time input. Is a value in the `0.0..=1.0` range.
#[derive(Debug, Clone, Copy)]
pub struct EasingTime(f32);

impl EasingTime {
    /// New easing time from calculated value.
    pub fn from_raw(raw: f32) -> EasingTime {
        EasingTime(raw.max(0.0).min(1.0))
    }

    /// New easing time from total `duration` and `elapsed` time.
    #[inline]
    pub fn new(duration: Duration, elapsed: Duration) -> EasingTime {
        if elapsed < duration {
            EasingTime(duration.as_secs_f32() / elapsed.as_secs_f32())
        } else {
            EasingTime(1.0)
        }
    }

    /// Gets the scale value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }
}

/// A [VarValue] that is the final value of a transition animation and
/// can be multiplied by a [EasingStep] to get an intermediary value of the
/// same type.
pub trait EasingVarValue: VarValue + Mul<EasingStep, Output = Self> {}

impl<T: VarValue + Mul<EasingStep, Output = Self>> EasingVarValue for T {}

impl Mul<EasingStep> for f32 {
    type Output = Self;

    fn mul(self, rhs: EasingStep) -> f32 {
        self * rhs.0
    }
}

impl Mul<EasingStep> for f64 {
    type Output = Self;

    fn mul(self, rhs: EasingStep) -> f64 {
        self * rhs.0 as f64
    }
}

macro_rules! mul_easing_step_for_small_int {
    ($($ty:ty),+) => {$(
        impl Mul<EasingStep> for $ty {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: EasingStep) -> Self {
                let r = f32::from(self) * rhs.0;

                let min = <$ty>::min_value();
                let min_f: f32 = min.into();
                if r < min_f {
                    return min;
                }

                let max = <$ty>::max_value();
                let max_f: f32 = max.into();
                if r > max_f {
                    return max;
                }

                r.round() as Self
            }
        }
    )+};
}

mul_easing_step_for_small_int!(u8, i8, u16, i16);

macro_rules! mul_easing_step_for_int  {
    ($($ty:ty),+) => {$(
        impl Mul<EasingStep> for $ty {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: EasingStep) -> Self {
                let r = f64::from(self) * rhs.0 as f64;

                let min = <$ty>::min_value();
                let min_f: f64 = min.into();
                if r < min_f {
                    return min;
                }

                let max = <$ty>::max_value();
                let max_f: f64 = max.into();
                if r > max_f {
                    return max;
                }

                r.round() as Self
            }
        }
    )+};
}

mul_easing_step_for_int!(u32, i32);

impl Mul<EasingStep> for u64 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: EasingStep) -> Self {
        todo!()
    }
}

/// Common easing functions.
pub mod easing {
    use super::{EasingStep, EasingTime};

    /// Simple linear transition, no easing, no acceleration.
    #[inline]
    pub fn linear(time: EasingTime) -> EasingStep {
        EasingStep(time.get())
    }

    #[inline]
    pub fn quad_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn quad_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Quadratic in/out.
    #[inline]
    pub fn quad(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn cubic_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn cubic_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Cubic in/out.
    #[inline]
    pub fn cubic(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn quart_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn quart_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Quartic in/out.
    #[inline]
    pub fn quart(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn quint_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn quint_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Quintic in/out.
    #[inline]
    pub fn quint(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn sine_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn sine_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Sinusoidal in/out.
    #[inline]
    pub fn sine(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn expo_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn expo_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Exponential in/out.
    #[inline]
    pub fn expo(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn circ_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn circ_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Circular in/out.
    #[inline]
    pub fn circ(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn back_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn back_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Back in/out.
    #[inline]
    pub fn back(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn elastic_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn elastic_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Elastic in/out.
    #[inline]
    pub fn elastic(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn bounce_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn bounce_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Bounce in/out.
    #[inline]
    pub fn bounce(time: EasingTime) -> EasingStep {
        todo!()
    }
}
